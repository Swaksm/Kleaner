; Crochets de l'installateur NSIS de Kleaner (voir tauri.conf.json > bundle > windows > nsis > installerHooks).

; Après une vraie désinstallation : page d'avis, une seule fois, rien d'obligatoire.
; - Lancée depuis Windows (ou « Retirer Kleaner »), la désinstallation tourne depuis une copie dans %TEMP%.
;   Pendant une réinstallation, l'installateur lance le désinstalleur sur place : pas de page.
; - Jamais en mode silencieux (winget, scripts).
; - Ouverte par explorer.exe, donc dans le navigateur habituel, sans les droits admin du désinstalleur.
!macro NSIS_HOOK_POSTUNINSTALL
  ${IfNot} ${Silent}
    StrLen $1 $TEMP
    StrCpy $0 $EXEDIR $1
    ${If} $0 == $TEMP
      Exec '"$WINDIR\explorer.exe" "https://swaks.org/kleaner/?bye#feedback"'
    ${EndIf}
  ${EndIf}
!macroend
