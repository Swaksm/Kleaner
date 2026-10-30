//! Point de restauration Windows avant un nettoyage lourd (case cochée par défaut dans le Nettoyage).
//! Windows n'en crée pas plus d'un par 24 h : un point plus récent que ça sert de filet, on n'en demande pas d'autre.
//! Le réglage qui lève cette limite est dans le registre : Kleaner n'y touche pas.

use crate::journal;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    Created,
    /// Point de moins de 24 h déjà là (secondes Unix).
    Recent { ts: u64 },
    /// Pas de point : protection du système coupée, service arrêté… `detail` = sortie de PowerShell.
    Failed { detail: String },
}

const SCRIPT: &str = "$ErrorActionPreference='Stop'; \
$last = Get-ComputerRestorePoint -ErrorAction SilentlyContinue | Sort-Object SequenceNumber | Select-Object -Last 1; \
if ($last) { $t = $last.ConvertToDateTime($last.CreationTime); \
  if (((Get-Date) - $t).TotalHours -lt 24) { \"recent $([DateTimeOffset]::new($t).ToUnixTimeSeconds())\"; exit } }; \
Checkpoint-Computer -Description 'Kleaner' -RestorePointType MODIFY_SETTINGS; \
$new = Get-ComputerRestorePoint | Sort-Object SequenceNumber | Select-Object -Last 1; \
if ($new -and (-not $last -or $new.SequenceNumber -gt $last.SequenceNumber)) { 'created' } else { 'none' }";

/// Dernière ligne de la sortie : « created », « recent <ts> », sinon échec (sortie complète gardée).
fn parse(ok: bool, output: &str) -> Outcome {
    let last = output.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("");
    match last.split_once(' ') {
        _ if ok && last == "created" => Outcome::Created,
        Some(("recent", ts)) if ok => ts.parse().map_or_else(|_| Outcome::Failed { detail: output.trim().into() }, |ts| Outcome::Recent { ts }),
        _ => Outcome::Failed { detail: output.trim().into() },
    }
}

/// Admin requis (comme les nettoyages lourds). Journalisé.
pub fn create(journal_dir: &Path) -> Outcome {
    let (ok, output) = crate::catalog::commands::powershell(SCRIPT);
    let outcome = parse(ok, &output);
    let mut j = journal::Entry::new("restore-point");
    j.output = Some(output);
    if let Outcome::Failed { detail } = &outcome {
        j.error = Some(detail.lines().last().unwrap_or("no restore point").into());
    }
    let _ = journal::append(journal_dir, &j);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_read_from_the_last_line() {
        assert_eq!(parse(true, "> powershell\ncreated\n"), Outcome::Created);
        assert_eq!(parse(true, "> powershell\nrecent 1761830000"), Outcome::Recent { ts: 1_761_830_000 });
        assert!(matches!(parse(true, "> powershell\nnone"), Outcome::Failed { .. }), "Windows n'a rien créé");
        assert!(matches!(parse(false, "> powershell\ncreated"), Outcome::Failed { .. }), "code de sortie en échec");
        assert!(matches!(parse(true, "recent pas-un-nombre"), Outcome::Failed { .. }));
    }
}
