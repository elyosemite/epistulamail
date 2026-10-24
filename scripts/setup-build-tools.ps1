# Baixa o Strawberry Perl portátil usado para compilar o OpenSSL embutido do SQLCipher (US3).
# Fica em .tools/ (ignorado pelo git); src-tauri/.cargo/config.toml aponta para ele.
$ErrorActionPreference = 'Stop'
$version = '5.42.3.1'
$url = "https://github.com/StrawberryPerl/Perl-Dist-Strawberry/releases/download/SP_54231_64bit/strawberry-perl-$version-64bit-portable.zip"
$sha256 = '6A081A811781C30ACA51DBC036AFD93092AF91E3297901F02C17043795A10690'
$root = Join-Path $PSScriptRoot '..\.tools'
$target = Join-Path $root 'strawberry'
if (Test-Path (Join-Path $target 'perl\bin\perl.exe')) { Write-Host 'Strawberry Perl já está em .tools/strawberry'; exit 0 }
New-Item -ItemType Directory -Force $root | Out-Null
$zip = Join-Path $root 'strawberry.zip'
Invoke-WebRequest $url -OutFile $zip
$actual = (Get-FileHash $zip -Algorithm SHA256).Hash
if ($actual -ne $sha256) { Remove-Item $zip; throw "SHA-256 inesperado: $actual" }
Expand-Archive $zip -DestinationPath $target
Remove-Item $zip
Write-Host 'Strawberry Perl instalado em .tools/strawberry'
