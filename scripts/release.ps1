# Prepara uma versão nova do EpistulaMail: sobe a versão nos três arquivos que o pipeline confere
# (package.json, src-tauri/Cargo.toml + Cargo.lock e src-tauri/tauri.conf.json) e abre a seção dela no
# CHANGELOG.md. Não faz commit nem tag: revise, escreva as notas e siga os comandos que ele mostra no fim.
#
#   ./scripts/release.ps1 1.0.2
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Read-Text($path) { [System.IO.File]::ReadAllText((Join-Path $root $path), $utf8) }
function Write-Text($path, $text) { [System.IO.File]::WriteAllText((Join-Path $root $path), $text, $utf8) }

# Troca a primeira ocorrência do padrão; falha se não achar, para nenhum arquivo ficar para trás.
function Set-Version($path, $pattern, $replacement) {
    $text = Read-Text $path
    $regex = [regex]::new($pattern, 'Multiline')
    if (-not $regex.IsMatch($text)) { throw "Versão não encontrada em $path" }
    Write-Text $path ($regex.Replace($text, $replacement, 1))
}

$current = (Read-Text 'src-tauri/tauri.conf.json' | ConvertFrom-Json).version
if ([version]$Version -le [version]$current) { throw "A versão nova ($Version) precisa ser maior que a atual ($current)." }
if (git -C $root tag --list "v$Version") { throw "A tag v$Version já existe." }

Set-Version 'package.json' '"version": "\d+\.\d+\.\d+"' "`"version`": `"$Version`""
Set-Version 'src-tauri/tauri.conf.json' '"version": "\d+\.\d+\.\d+"' "`"version`": `"$Version`""
Set-Version 'src-tauri/Cargo.toml' '^version = "\d+\.\d+\.\d+"' "version = `"$Version`""
Set-Version 'src-tauri/Cargo.lock' '(name = "epistula"\r?\nversion = )"\d+\.\d+\.\d+"' "`${1}`"$Version`""

$changelog = Read-Text 'CHANGELOG.md'
if ($changelog -match "(?m)^## \[$([regex]::Escape($Version))\]") {
    Write-Host "CHANGELOG.md já tem a seção [$Version]."
} else {
    $date = Get-Date -Format 'yyyy-MM-dd'
    $section = "## [$Version] - $date`n`n### Mudou`n`n- `n`n"
    $at = [regex]::Match($changelog, '(?m)^## \[')
    if (-not $at.Success) { throw 'CHANGELOG.md sem nenhuma seção de versão.' }
    Write-Text 'CHANGELOG.md' ($changelog.Insert($at.Index, $section))
    Write-Host "CHANGELOG.md: seção [$Version] aberta; escreva o que mudou."
}

Write-Host ""
Write-Host "Versão $current -> $Version em package.json, tauri.conf.json, Cargo.toml e Cargo.lock."
Write-Host "Depois de escrever as notas no CHANGELOG.md:"
Write-Host "  git add -A; git commit -m `"release: $Version`""
Write-Host "  git push origin main"
Write-Host "  git tag -a v$Version -m `"EpistulaMail $Version`"; git push origin v$Version"
Write-Host "O pipeline cria a release em rascunho; publique-a no GitHub para o site e o atualizador a verem."
