; Ganchos do instalador NSIS (tauri.conf.json → bundle.windows.nsis.installerHooks).
;
; Até a 1.0.1 o produto se chamava "Epistula"; o NSIS do Tauri guarda a instalação em chaves com o nome do
; produto, então o instalador do "EpistulaMail" não acharia a anterior e as duas ficariam lado a lado. Antes de
; copiar os arquivos, remove a instalação "Epistula" em silêncio. Os dados (banco, configurações) ficam em
; %APPDATA%\br.com.epistula.desktop, que é do identificador e não do nome, e o desinstalador silencioso não os
; apaga; as credenciais no cofre do Windows também ficam.

!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\Epistula" "UninstallString"
  ${If} $R0 != ""
    ReadRegStr $R1 SHCTX "Software\Epistula\Epistula" ""
    ${If} $R1 != ""
    ${AndIf} ${FileExists} "$R1\uninstall.exe"
      DetailPrint "Removendo a versão anterior (Epistula) de $R1"
      ExecWait '"$R1\uninstall.exe" /S _?=$R1'
      Delete "$R1\uninstall.exe"
      RMDir "$R1"
    ${EndIf}
  ${EndIf}
!macroend
