; Crochets de l'installateur NSIS de Kleaner (voir tauri.conf.json > bundle > windows > nsis > installerHooks).

; Après une vraie désinstallation :
; - Lancée depuis Windows (ou « Retirer Kleaner »), la désinstallation tourne depuis une copie dans %TEMP%.
;   Pendant une réinstallation ou une mise à jour, l'installateur lance le désinstalleur sur place : rien de tout ça.
; - La tâche « Démarrer avec Windows » est retirée si elle lance l'exe de ce dossier d'installation
;   (celle d'un Kleaner portable reste), sinon elle échouerait à chaque ouverture de session.
; - Page d'avis, une seule fois, rien d'obligatoire, jamais en mode silencieux (winget, scripts).
;   Ouverte par explorer.exe, donc dans le navigateur habituel, sans les droits admin du désinstalleur.
!macro NSIS_HOOK_POSTUNINSTALL
  StrLen $1 $TEMP
  StrCpy $0 $EXEDIR $1
  ${If} $0 == $TEMP
    nsExec::Exec `"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "$$t = Get-ScheduledTask -TaskName 'Kleaner' -ErrorAction SilentlyContinue; if ($$t -and $$t.Actions[0].Execute -like '$INSTDIR\*') { Unregister-ScheduledTask -TaskName 'Kleaner' -Confirm:$$false }"`
    Pop $2
    ${IfNot} ${Silent}
      Exec '"$WINDIR\explorer.exe" "https://swaks.org/kleaner/?bye#feedback"'
    ${EndIf}
  ${EndIf}
!macroend
