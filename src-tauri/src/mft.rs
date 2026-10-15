//! Scan rapide d'un volume NTFS : au lieu de lister les dossiers un par un, on lit d'un bloc la table
//! des fichiers du volume (MFT) et on en tire noms, tailles et dates. Lecture seule, droits admin requis.
//!
//! Tout échec (pas admin, pas NTFS, MFT fragmentée sur des enregistrements d'extension, résultat
//! incohérent avec l'espace utilisé du disque…) renvoie `None` : le scan classique prend le relais.
//! Le résultat alimente le même parcours (`scan::walk`) : mêmes règles, mêmes listes, seul le listage change.
//! `KLEANER_NO_MFT=1` désactive ce mode.

use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// Numéro d'enregistrement du dossier racine d'un volume NTFS.
pub const ROOT: u32 = 5;
/// Les enregistrements 0 à 23 sont les fichiers système ($MFT, $LogFile…) : le listage normal ne les montre pas.
const FIRST_USER_RECORD: u32 = 24;
const BATCH: usize = 64 << 20;
const RECORDS_PER_TASK: usize = 4096;
/// Écart entre 1601 (FILETIME) et 1970 (Unix), en unités de 100 ns.
const EPOCH_DIFF: u64 = 116_444_736_000_000_000;

const IN_USE: u8 = 1;
const DIR: u8 = 2;
const EXT: u8 = 4;
const HAS_SIZE: u8 = 8;

#[derive(Clone, Copy, Default, Debug)]
struct Meta {
    size: u64,
    mtime: u32,
    attrs: u32,
    flags: u8,
}

#[derive(Clone, Copy, Debug)]
struct NameRec {
    frn: u32,
    parent: u32,
    off: u32,
    len: u16,
}

#[derive(Default)]
struct Chunk {
    meta: Vec<Meta>,
    names: Vec<NameRec>,
    arena: Vec<u16>,
    /// Tailles trouvées dans des enregistrements d'extension : (enregistrement de base, taille).
    ext_sizes: Vec<(u32, u64)>,
}

#[derive(Clone, Copy, Default)]
struct Node {
    frn: u32,
    size: u64,
    mtime: u32,
    attrs: u32,
    dir: bool,
    off: u32,
    len: u16,
}

/// Un élément d'un dossier, tel que le listage normal le donnerait.
pub struct Child {
    pub frn: u32,
    pub name: String,
    pub is_dir: bool,
    pub attrs: u32,
    pub len: u64,
    pub modified: u64,
}

/// Tous les fichiers et dossiers d'un volume, classés par dossier parent.
pub struct Index {
    nodes: Vec<Node>,
    starts: Vec<u32>,
    arena: Vec<u16>,
    /// Somme des tailles des fichiers (les liens physiques comptent chacun).
    pub bytes: u64,
}

impl Index {
    /// Assemble l'index à partir de ce qui a été lu dans la MFT.
    fn from_parts(meta: &[Meta], names: &[NameRec], arena: Vec<u16>, ext_sizes: &[(u32, u64)]) -> Index {
        let records = meta.len();
        let mut ext: HashMap<u32, u64> = HashMap::new();
        for &(base, size) in ext_sizes {
            ext.entry(base).or_insert(size);
        }
        let mut nodes: Vec<(u32, Node)> = Vec::with_capacity(names.len());
        let mut bytes = 0u64;
        for n in names {
            let Some(m) = meta.get(n.frn as usize) else { continue };
            if m.flags & IN_USE == 0 || m.flags & EXT != 0 || n.frn == ROOT || n.frn < FIRST_USER_RECORD || n.parent as usize >= records {
                continue;
            }
            let dir = m.flags & DIR != 0;
            let size = if dir {
                0
            } else if m.flags & HAS_SIZE != 0 {
                m.size
            } else {
                ext.get(&n.frn).copied().unwrap_or(0)
            };
            bytes += size;
            nodes.push((n.parent, Node { frn: n.frn, size, mtime: m.mtime, attrs: m.attrs, dir, off: n.off, len: n.len }));
        }
        // Tri par dossier parent (tri par comptage : les numéros sont denses).
        let mut starts = vec![0u32; records + 2];
        for (p, _) in &nodes {
            starts[*p as usize + 1] += 1;
        }
        for i in 1..starts.len() {
            starts[i] += starts[i - 1];
        }
        let mut next = starts.clone();
        let mut sorted = vec![Node::default(); nodes.len()];
        for (p, n) in nodes {
            sorted[next[p as usize] as usize] = n;
            next[p as usize] += 1;
        }
        Index { nodes: sorted, starts, arena, bytes }
    }

    pub fn children(&self, dir: u32) -> Vec<Child> {
        let (a, b) = match (self.starts.get(dir as usize), self.starts.get(dir as usize + 1)) {
            (Some(&a), Some(&b)) => (a as usize, b as usize),
            _ => return Vec::new(),
        };
        self.nodes[a..b]
            .iter()
            .map(|n| Child {
                frn: n.frn,
                name: String::from_utf16_lossy(&self.arena[n.off as usize..n.off as usize + n.len as usize]),
                is_dir: n.dir,
                attrs: n.attrs,
                len: n.size,
                modified: n.mtime as u64,
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

// ---------- Lecture des octets ----------

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

fn filetime_to_unix(ft: u64) -> u32 {
    (ft.saturating_sub(EPOCH_DIFF) / 10_000_000).min(u32::MAX as u64) as u32
}

/// Remet en place les 2 derniers octets de chaque secteur de 512 octets (protection contre les écritures partielles).
fn apply_fixups(rec: &mut [u8]) -> bool {
    if rec.len() < 0x30 || &rec[..4] != b"FILE" {
        return false;
    }
    let usa = u16_at(rec, 4) as usize;
    let count = u16_at(rec, 6) as usize;
    if count < 2 || usa + count * 2 > rec.len() || count * 512 > rec.len() + 512 {
        return false;
    }
    let check = [rec[usa], rec[usa + 1]];
    for i in 1..count {
        let end = i * 512;
        if end > rec.len() || rec[end - 2..end] != check {
            return false;
        }
        rec[end - 2] = rec[usa + 2 * i];
        rec[end - 1] = rec[usa + 2 * i + 1];
    }
    true
}

/// Lit un enregistrement : date, attributs, noms (hors noms DOS 8.3) et taille du flux de données principal.
fn parse_record(rec: &mut [u8], frn: u32, out: &mut Chunk) {
    let mut meta = Meta::default();
    if apply_fixups(rec) && u16_at(rec, 0x16) & 1 != 0 {
        let flags = u16_at(rec, 0x16);
        meta.flags = IN_USE | if flags & 2 != 0 { DIR } else { 0 };
        let base = (u64_at(rec, 0x20) & 0xFFFF_FFFF_FFFF) as u32;
        if base != 0 {
            meta.flags |= EXT;
        }
        let used = (u32_at(rec, 0x18) as usize).min(rec.len());
        let mut pos = u16_at(rec, 0x14) as usize;
        while pos + 16 <= used {
            let ty = u32_at(rec, pos);
            let len = u32_at(rec, pos + 4) as usize;
            if ty == 0xFFFF_FFFF || len < 16 || pos + len > used {
                break;
            }
            let non_resident = rec[pos + 8] != 0;
            let unnamed = rec[pos + 9] == 0;
            match ty {
                // $STANDARD_INFORMATION : dernière écriture (+8) et attributs (+0x20).
                0x10 if !non_resident => {
                    let v = pos + u16_at(rec, pos + 0x14) as usize;
                    if u32_at(rec, pos + 0x10) >= 0x24 && v + 0x24 <= used {
                        meta.mtime = filetime_to_unix(u64_at(rec, v + 8));
                        meta.attrs = u32_at(rec, v + 0x20);
                    }
                }
                // $FILE_NAME : un par nom ; l'espace de noms 2 est le nom court DOS, à ignorer.
                // Un fichier avec beaucoup de liens physiques (WinSxS…) garde une partie de ses noms dans des enregistrements d'extension.
                0x30 if !non_resident => {
                    let v = pos + u16_at(rec, pos + 0x14) as usize;
                    if u32_at(rec, pos + 0x10) >= 0x42 && v + 0x42 <= used {
                        let name_len = rec[v + 0x40] as usize;
                        let namespace = rec[v + 0x41];
                        if namespace != 2 && name_len > 0 && v + 0x42 + name_len * 2 <= used {
                            let off = out.arena.len() as u32;
                            out.arena.extend((0..name_len).map(|i| u16_at(rec, v + 0x42 + i * 2)));
                            let parent = (u64_at(rec, v) & 0xFFFF_FFFF_FFFF) as u32;
                            let owner = if base != 0 { base } else { frn };
                            out.names.push(NameRec { frn: owner, parent, off, len: name_len as u16 });
                        }
                    }
                }
                // $DATA sans nom : taille du fichier (le premier morceau d'un flux fragmenté porte la taille totale).
                0x80 if unnamed => {
                    let size = if non_resident {
                        (u64_at(rec, pos + 0x10) == 0).then(|| u64_at(rec, pos + 0x30))
                    } else {
                        Some(u32_at(rec, pos + 0x10) as u64)
                    };
                    if let Some(size) = size {
                        if base != 0 {
                            out.ext_sizes.push((base, size));
                        } else {
                            meta.size = size;
                            meta.flags |= HAS_SIZE;
                        }
                    }
                }
                _ => {}
            }
            pos += len;
        }
    }
    out.meta.push(meta);
}

/// Parcourt `buf` enregistrement par enregistrement ; `first` : numéro du premier.
fn parse_chunk(buf: &mut [u8], first: u32, record_size: usize) -> Chunk {
    let mut out = Chunk::default();
    for (i, rec) in buf.chunks_exact_mut(record_size).enumerate() {
        parse_record(rec, first + i as u32, &mut out);
    }
    out
}

// ---------- Fragments de la MFT ----------

fn le_unsigned(b: &[u8]) -> u64 {
    b.iter().rev().fold(0, |acc, &x| (acc << 8) | x as u64)
}

fn le_signed(b: &[u8]) -> i64 {
    let v = le_unsigned(b) as i64;
    let shift = 64 - 8 * b.len() as u32;
    (v << shift) >> shift
}

/// Liste de fragments (data runs) : (premier cluster, nombre de clusters). Les fragments « creux » sont refusés.
fn parse_runs(data: &[u8]) -> Option<Vec<(u64, u64)>> {
    let (mut runs, mut pos, mut lcn) = (Vec::new(), 0usize, 0i64);
    while pos < data.len() && data[pos] != 0 {
        let (len_size, off_size) = ((data[pos] & 0xF) as usize, (data[pos] >> 4) as usize);
        pos += 1;
        if len_size == 0 || len_size > 8 || off_size == 0 || off_size > 8 || pos + len_size + off_size > data.len() {
            return None;
        }
        let len = le_unsigned(&data[pos..pos + len_size]);
        pos += len_size;
        lcn += le_signed(&data[pos..pos + off_size]);
        pos += off_size;
        if lcn < 0 {
            return None;
        }
        runs.push((lcn as u64, len));
    }
    Some(runs)
}

/// Fragments et taille réelle de la MFT, lus dans son propre enregistrement (le numéro 0).
/// `None` si la MFT est trop fragmentée pour tenir dans cet enregistrement (liste d'attributs).
fn mft_extents(rec: &mut [u8]) -> Option<(Vec<(u64, u64)>, u64)> {
    if !apply_fixups(rec) {
        return None;
    }
    let used = (u32_at(rec, 0x18) as usize).min(rec.len());
    let mut pos = u16_at(rec, 0x14) as usize;
    let mut found = None;
    while pos + 16 <= used {
        let ty = u32_at(rec, pos);
        let len = u32_at(rec, pos + 4) as usize;
        if ty == 0xFFFF_FFFF || len < 16 || pos + len > used {
            break;
        }
        if ty == 0x80 && rec[pos + 8] != 0 && rec[pos + 9] == 0 && len >= 0x40 {
            let run_off = u16_at(rec, pos + 0x20) as usize;
            if run_off < len {
                found = Some((parse_runs(&rec[pos + run_off..pos + len])?, u64_at(rec, pos + 0x30)));
            }
        }
        pos += len;
    }
    found
}

// ---------- Volume ----------

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VolumeData {
    serial: i64,
    sectors: i64,
    clusters: i64,
    free: i64,
    reserved: i64,
    bytes_per_sector: u32,
    bytes_per_cluster: u32,
    bytes_per_record: u32,
    clusters_per_record: u32,
    mft_valid: i64,
    mft_start: i64,
    mft2_start: i64,
    zone_start: i64,
    zone_end: i64,
}

/// FSCTL_GET_NTFS_VOLUME_DATA ; échoue sur un volume qui n'est pas NTFS.
fn volume_data(f: &File) -> Option<VolumeData> {
    use windows_sys::Win32::System::IO::DeviceIoControl;
    let mut buf = [0u64; 16];
    let mut returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(f.as_raw_handle().cast(), 0x0009_0064, std::ptr::null(), 0, buf.as_mut_ptr().cast(), 128, &mut returned, std::ptr::null_mut())
    };
    (ok != 0 && returned as usize >= size_of::<VolumeData>()).then(|| unsafe { std::ptr::read_unaligned(buf.as_ptr().cast::<VolumeData>()) })
}

fn read_at(f: &File, offset: u64, buf: &mut [u8]) -> bool {
    let mut done = 0;
    while done < buf.len() {
        match f.seek_read(&mut buf[done..], offset + done as u64) {
            Ok(0) | Err(_) => return false,
            Ok(n) => done += n,
        }
    }
    true
}

/// Lit `len` octets de la MFT (flux logique) à partir de `logical`, à travers ses fragments.
/// `logical` et `len` sont des multiples de la taille d'un cluster.
fn read_logical(f: &File, extents: &[(u64, u64)], cluster: u64, logical: u64, buf: &mut [u8]) -> bool {
    let (mut start, mut filled) = (0u64, 0usize);
    for &(lcn, clusters) in extents {
        let (ext_len, ext_start) = (clusters * cluster, start);
        start += ext_len;
        let end = logical + buf.len() as u64;
        if ext_start + ext_len <= logical || ext_start >= end {
            continue;
        }
        let from = logical.max(ext_start);
        let to = end.min(ext_start + ext_len);
        let n = (to - from) as usize;
        if !read_at(f, lcn * cluster + (from - ext_start), &mut buf[(from - logical) as usize..(from - logical) as usize + n]) {
            return false;
        }
        filled += n;
    }
    filled == buf.len()
}

/// Construit l'index du volume `root` (ex. `C:\`). `used` : octets utilisés du volume, pour vérifier la cohérence.
pub fn build(root: &Path, cancel: &AtomicBool, used: u64) -> Option<Index> {
    if std::env::var_os("KLEANER_NO_MFT").is_some() {
        return None;
    }
    let letter = root.to_string_lossy().chars().next().filter(char::is_ascii_alphabetic)?;
    let f = File::options().read(true).share_mode(7).open(format!(r"\\.\{letter}:")).ok()?;
    let vol = volume_data(&f)?;
    let (record, cluster) = (vol.bytes_per_record as usize, vol.bytes_per_cluster as u64);
    if !(512..=65_536).contains(&record) || !record.is_power_of_two() || cluster < 512 || !cluster.is_power_of_two() || vol.mft_start <= 0 {
        return None;
    }
    // Enregistrement 0 : la MFT se décrit elle-même.
    let first = (record as u64).div_ceil(cluster) * cluster;
    let mut rec0 = vec![0u8; first as usize];
    if !read_at(&f, vol.mft_start as u64 * cluster, &mut rec0) {
        return None;
    }
    let (extents, real) = mft_extents(&mut rec0[..record])?;
    let allocated: u64 = extents.iter().map(|e| e.1 * cluster).sum();
    let valid = if vol.mft_valid > 0 { real.min(vol.mft_valid as u64) } else { real };
    if allocated < valid || valid < record as u64 {
        return None; // fragments manquants : MFT décrite par des enregistrements d'extension
    }
    let records = (valid / record as u64) as usize;
    if records as u64 >= u32::MAX as u64 {
        return None;
    }

    let (mut meta, mut names, mut arena, mut ext_sizes) = (Vec::with_capacity(records), Vec::new(), Vec::<u16>::new(), Vec::new());
    let mut offset = 0u64;
    while offset < valid {
        if cancel.load(Relaxed) {
            return None;
        }
        let want = (BATCH as u64).min(valid - offset);
        let read_len = want.div_ceil(cluster) * cluster;
        let mut buf = vec![0u8; read_len.min(allocated - offset) as usize];
        if !read_logical(&f, &extents, cluster, offset, &mut buf) {
            return None;
        }
        let usable = (want as usize / record) * record;
        let first_record = (offset / record as u64) as u32;
        let parts: Vec<Chunk> = buf[..usable]
            .par_chunks_mut(record * RECORDS_PER_TASK)
            .enumerate()
            .map(|(i, part)| parse_chunk(part, first_record + (i * RECORDS_PER_TASK) as u32, record))
            .collect();
        for mut p in parts {
            let base = arena.len() as u32;
            for n in &mut p.names {
                n.off += base;
            }
            meta.append(&mut p.meta);
            names.append(&mut p.names);
            arena.append(&mut p.arena);
            ext_sizes.append(&mut p.ext_sizes);
        }
        offset += usable as u64;
        if usable == 0 {
            break;
        }
    }
    let index = Index::from_parts(&meta, &names, arena, &ext_sizes);
    // Cohérence : les fichiers doivent représenter l'essentiel de l'espace utilisé du disque.
    let plausible = !index.is_empty() && !index.children(ROOT).is_empty() && (used == 0 || index.bytes.saturating_mul(100) >= used.saturating_mul(60));
    plausible.then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: usize = 1024;

    fn ft(unix: u64) -> u64 {
        unix * 10_000_000 + EPOCH_DIFF
    }

    fn header(attr_len: usize, flags: u16, base: u64, rec: &mut Vec<u8>) {
        rec.resize(SIZE, 0);
        rec[0..4].copy_from_slice(b"FILE");
        rec[4..6].copy_from_slice(&0x30u16.to_le_bytes()); // USA
        rec[6..8].copy_from_slice(&3u16.to_le_bytes()); // 2 secteurs + la valeur de contrôle
        rec[0x14..0x16].copy_from_slice(&0x38u16.to_le_bytes());
        rec[0x16..0x18].copy_from_slice(&flags.to_le_bytes());
        rec[0x18..0x1C].copy_from_slice(&((0x38 + attr_len + 4) as u32).to_le_bytes());
        rec[0x1C..0x20].copy_from_slice(&(SIZE as u32).to_le_bytes());
        rec[0x20..0x28].copy_from_slice(&base.to_le_bytes());
    }

    fn resident(ty: u32, value: &[u8]) -> Vec<u8> {
        let len = (0x18 + value.len()).div_ceil(8) * 8;
        let mut a = vec![0u8; len];
        a[0..4].copy_from_slice(&ty.to_le_bytes());
        a[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        a[0x10..0x14].copy_from_slice(&(value.len() as u32).to_le_bytes());
        a[0x14..0x16].copy_from_slice(&0x18u16.to_le_bytes());
        a[0x18..0x18 + value.len()].copy_from_slice(value);
        a
    }

    fn std_info(mtime: u64, attrs: u32) -> Vec<u8> {
        let mut v = vec![0u8; 0x48];
        v[8..16].copy_from_slice(&ft(mtime).to_le_bytes());
        v[0x20..0x24].copy_from_slice(&attrs.to_le_bytes());
        resident(0x10, &v)
    }

    fn file_name(parent: u32, name: &str, namespace: u8) -> Vec<u8> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let mut v = vec![0u8; 0x42 + units.len() * 2];
        v[0..8].copy_from_slice(&(parent as u64 | (1 << 48)).to_le_bytes()); // numéro de séquence dans les 16 bits hauts
        v[0x40] = units.len() as u8;
        v[0x41] = namespace;
        for (i, u) in units.iter().enumerate() {
            v[0x42 + i * 2..0x44 + i * 2].copy_from_slice(&u.to_le_bytes());
        }
        resident(0x30, &v)
    }

    fn data_resident(size: u32) -> Vec<u8> {
        let mut a = resident(0x80, &vec![0u8; size as usize]);
        a[0x10..0x14].copy_from_slice(&size.to_le_bytes());
        a
    }

    fn data_non_resident(real: u64, start_vcn: u64) -> Vec<u8> {
        let mut a = vec![0u8; 0x48];
        a[0..4].copy_from_slice(&0x80u32.to_le_bytes());
        a[4..8].copy_from_slice(&0x48u32.to_le_bytes());
        a[8] = 1;
        a[0x10..0x18].copy_from_slice(&start_vcn.to_le_bytes());
        a[0x20..0x22].copy_from_slice(&0x40u16.to_le_bytes());
        a[0x28..0x30].copy_from_slice(&(real.div_ceil(4096) * 4096).to_le_bytes());
        a[0x30..0x38].copy_from_slice(&real.to_le_bytes());
        a
    }

    /// Enregistrement complet de 1024 octets, avec les protections de fin de secteur en place.
    fn record(flags: u16, base: u64, attrs: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = attrs.concat();
        let mut rec = Vec::new();
        header(body.len(), flags, base, &mut rec);
        rec[0x38..0x38 + body.len()].copy_from_slice(&body);
        rec[0x38 + body.len()..0x38 + body.len() + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let check = [0xEF, 0xBE];
        rec[0x30..0x32].copy_from_slice(&check);
        for i in 1..3 {
            let end = i * 512;
            let original = [rec[end - 2], rec[end - 1]];
            rec[0x30 + 2 * i..0x32 + 2 * i].copy_from_slice(&original);
            rec[end - 2..end].copy_from_slice(&check);
        }
        rec
    }

    fn parse(mut rec: Vec<u8>, frn: u32) -> Chunk {
        let mut out = Chunk::default();
        parse_record(&mut rec, frn, &mut out);
        out
    }

    #[test]
    fn file_record_gives_name_size_date_and_attributes() {
        let rec = record(1, 0, &[std_info(1_700_000_000, 0x20), file_name(5, "PROGRA~1", 2), file_name(5, "Program Files", 1), data_non_resident(5_000_000_000, 0)]);
        let c = parse(rec, 40);
        assert_eq!(c.names.len(), 1, "le nom court DOS est ignoré");
        let n = c.names[0];
        assert_eq!((n.frn, n.parent, n.len), (40, 5, 13));
        assert_eq!(String::from_utf16_lossy(&c.arena[n.off as usize..][..n.len as usize]), "Program Files");
        let m = c.meta[0];
        assert_eq!((m.size, m.mtime, m.attrs), (5_000_000_000, 1_700_000_000, 0x20));
        assert!(m.flags & IN_USE != 0 && m.flags & DIR == 0 && m.flags & HAS_SIZE != 0);
    }

    #[test]
    fn names_stored_in_an_extension_record_belong_to_the_base_file() {
        // Enregistrement d'extension (base = 40) : un nom de plus, dans un autre dossier.
        let c = parse(record(1, 40, &[file_name(77, "hardlink.dll", 1)]), 900);
        assert_eq!(c.names.len(), 1);
        assert_eq!((c.names[0].frn, c.names[0].parent), (40, 77));
        assert!(c.meta[0].flags & EXT != 0, "l'enregistrement d'extension n'est pas un fichier à lui seul");
    }

    #[test]
    fn small_files_keep_their_data_in_the_record() {
        let c = parse(record(1, 0, &[std_info(1, 0), file_name(5, "a.txt", 1), data_resident(300)]), 30);
        assert_eq!(c.meta[0].size, 300);
    }

    #[test]
    fn directories_and_unused_records() {
        let dir = parse(record(3, 0, &[std_info(1, 0x10), file_name(5, "Users", 1)]), 31);
        assert!(dir.meta[0].flags & DIR != 0);
        let free = parse(record(0, 0, &[file_name(5, "deleted", 1)]), 32);
        assert!(free.meta[0].flags & IN_USE == 0 && free.names.is_empty(), "enregistrement libre : ignoré");
    }

    #[test]
    fn torn_write_is_rejected() {
        let mut rec = record(1, 0, &[file_name(5, "a", 1)]);
        rec[510] ^= 0xFF; // la fin du premier secteur ne vaut plus la valeur de contrôle
        let c = parse(rec, 33);
        assert!(c.meta[0].flags & IN_USE == 0 && c.names.is_empty());
    }

    #[test]
    fn named_streams_and_later_fragments_do_not_set_the_size() {
        // Seule la partie qui commence au cluster virtuel 0 porte la taille totale.
        let c = parse(record(1, 0, &[file_name(5, "big.bin", 1), data_non_resident(777, 100)]), 34);
        assert_eq!(c.meta[0].flags & HAS_SIZE, 0);
    }

    #[test]
    fn index_lists_children_and_hides_system_records() {
        let mut meta = vec![Meta::default(); 60];
        let mut arena: Vec<u16> = Vec::new();
        let mut names = Vec::new();
        let mut add = |frn: u32, parent: u32, name: &str, dir: bool, size: u64, has_size: bool| {
            let units: Vec<u16> = name.encode_utf16().collect();
            names.push(NameRec { frn, parent, off: arena.len() as u32, len: units.len() as u16 });
            arena.extend(units);
            meta[frn as usize] = Meta { size, mtime: 7, attrs: 0, flags: IN_USE | if dir { DIR } else { 0 } | if has_size { HAS_SIZE } else { 0 } };
        };
        add(0, 5, "$MFT", false, 9_999, true); // fichier système : masqué
        add(40, 5, "Users", true, 0, false);
        add(41, 40, "a.bin", false, 1_000, true);
        add(42, 40, "b.bin", false, 0, false); // taille dans un enregistrement d'extension
        add(43, 40, "link.bin", false, 1_000, true); // lien physique : un nom de plus
        names.push(NameRec { frn: 41, parent: 5, off: 9, len: 5 }); // même enregistrement, second parent (« a.bin »)
        let ix = Index::from_parts(&meta, &names, arena, &[(42, 2_500)]);
        let root: Vec<String> = ix.children(5).into_iter().map(|c| c.name).collect();
        assert!(root.contains(&"Users".into()) && !root.contains(&"$MFT".into()));
        let users = ix.children(40);
        assert_eq!(users.len(), 3);
        let size = |n: &str| users.iter().find(|c| c.name == n).unwrap().len;
        assert_eq!((size("a.bin"), size("b.bin"), size("link.bin")), (1_000, 2_500, 1_000));
        assert!(ix.children(41).is_empty() && ix.children(9_999).is_empty());
        assert_eq!(ix.bytes, 1_000 + 2_500 + 1_000 + 1_000);
    }

    #[test]
    fn runs_are_decoded() {
        // 0x18 clusters au cluster 0x5634, puis 0x10 clusters 0x30 clusters plus bas (décalage négatif).
        let data = [0x21, 0x18, 0x34, 0x56, 0x11, 0x10, 0xD0, 0x00];
        assert_eq!(parse_runs(&data), Some(vec![(0x5634, 0x18), (0x5634 - 0x30, 0x10)]));
        assert_eq!(parse_runs(&[0x01, 0x05, 0x00]), None, "fragment creux refusé");
        assert_eq!(le_signed(&[0xD0]), -48);
    }

    #[test]
    fn mft_describes_itself() {
        let mut attr = data_non_resident(1024 * 100, 0);
        attr[0x40..0x45].copy_from_slice(&[0x21, 0x19, 0x00, 0x10, 0x00]); // 0x19 clusters au cluster 0x1000
        let mut rec = record(1, 0, &[std_info(1, 0x6), attr]);
        let (extents, real) = mft_extents(&mut rec).unwrap();
        assert_eq!((real, extents), (1024 * 100, vec![(0x1000, 0x19)]));
    }

    #[test]
    fn filetime_conversion_clamps() {
        assert_eq!(filetime_to_unix(ft(1_700_000_000)), 1_700_000_000);
        assert_eq!(filetime_to_unix(0), 0);
    }

    /// Compare le scan rapide et le scan classique sur le vrai disque C: (lance-le dans un terminal administrateur) :
    /// `cargo test --manifest-path src-tauri/Cargo.toml --release mft::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        use crate::scan::{self, Progress};
        use std::path::PathBuf;
        assert!(crate::elevation::is_elevated(), "terminal administrateur requis");
        let root = PathBuf::from(r"C:\");
        let cancel = AtomicBool::new(false);
        let t = std::time::Instant::now();
        let used = scan::drive_space(&root).map_or(0, |d| d.total - d.free);
        let ix = build(&root, &cancel, used).expect("index MFT");
        println!("index MFT : {:?}, {} élément(s) à la racine, {:.1} Go de fichiers", t.elapsed(), ix.children(ROOT).len(), ix.bytes as f64 / 1e9);

        let fast = scan::scan(std::slice::from_ref(&root), &cancel, &Progress::default());
        // Test seul, un seul fil : on désactive le scan rapide pour mesurer le scan classique.
        unsafe { std::env::set_var("KLEANER_NO_MFT", "1") };
        let t = std::time::Instant::now();
        let slow = scan::scan(&[root], &cancel, &Progress::default());
        println!("classique : {:?}, rapide : {} ms", t.elapsed(), fast.duration_ms);
        println!("rapide {} fichiers {:.2} Go · classique {} fichiers {:.2} Go · rapide actif : {}", fast.files, fast.bytes as f64 / 1e9, slow.files, slow.bytes as f64 / 1e9, fast.fast);
        // Où sont les écarts ? Dossiers de plus de 64 Mo, du plus grand écart au plus petit.
        let sizes = |r: &scan::ScanResult| r.tree.iter().map(|i| (i.path.clone(), i.size)).collect::<std::collections::HashMap<_, _>>();
        let (f, s) = (sizes(&fast), sizes(&slow));
        let mut keys: Vec<&String> = f.keys().chain(s.keys()).collect();
        keys.sort();
        keys.dedup();
        let mut diffs: Vec<(u64, &String, u64, u64)> =
            keys.into_iter().map(|k| (f.get(k).copied().unwrap_or(0), s.get(k).copied().unwrap_or(0), k)).map(|(a, b, k)| (a.abs_diff(b), k, a, b)).collect();
        diffs.sort_by_key(|d| std::cmp::Reverse(d.0));
        for (d, k, a, b) in diffs.iter().take(25) {
            println!("écart {:7.2} Go  rapide {:7.2}  classique {:7.2}  {k}", *d as f64 / 1e9, *a as f64 / 1e9, *b as f64 / 1e9);
        }
        // Le scan classique compte deux fois les dossiers virtuels des applis du Store (MSIX : Roaming\Claude, pnpm…),
        // une fois dans Packages et une fois via le recouvrement. Mesuré : 3 % de l'espace, 5 % des fichiers. Tolérance : 8 %.
        let close = |a: u64, b: u64| a.abs_diff(b) * 100 <= a.max(b) * 8;
        assert!(fast.fast, "le scan rapide ne s'est pas activé");
        assert!(close(fast.files, slow.files) && close(fast.bytes, slow.bytes), "écart de plus de 8 % entre les deux scans");
    }
}
