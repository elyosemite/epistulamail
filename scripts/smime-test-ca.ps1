# CA de teste para o S/MIME (US14): testar sem comprar certificado. Usa o openssl que vem com o Git para Windows.
# A CA fica em %LOCALAPPDATA%\Epistula\test-ca, com a chave sem senha: serve só para testes, nunca para mensagens de verdade.
#
#   Assinar o pedido feito no Epistula (Configurações → Criptografia → Pedir um certificado → Salvar .csr):
#     .\scripts\smime-test-ca.ps1 -Csr .\Yuri-Melo-pedido.csr
#   Criar o certificado de um contato de teste (.p12, para importar no Thunderbird ou no Outlook):
#     .\scripts\smime-test-ca.ps1 -Person ana@exemplo.com -Name 'Ana Teste'
#   Confiar na CA de teste no Windows (só no seu usuário) e, depois, deixar de confiar:
#     .\scripts\smime-test-ca.ps1 -InstallRoot
#     .\scripts\smime-test-ca.ps1 -RemoveRoot
param(
    [string]$Csr,
    [string]$Person,
    [string]$Name = 'Contato de teste',
    [int]$Days = 365,
    [switch]$InstallRoot,
    [switch]$RemoveRoot
)
$ErrorActionPreference = 'Stop'
$caName = 'CA de teste do Epistula'
$openssl = @("$env:ProgramFiles\Git\usr\bin\openssl.exe", "$env:ProgramFiles\Git\mingw64\bin\openssl.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $openssl) { throw 'openssl não encontrado: instale o Git para Windows ou ponha o openssl no PATH.' }
$dir = Join-Path $env:LOCALAPPDATA 'Epistula\test-ca'
$caKey = Join-Path $dir 'ca.key'
$caCrt = Join-Path $dir 'ca.crt'

function Invoke-OpenSsl {
    & $openssl @args
    if ($LASTEXITCODE -ne 0) { throw "openssl falhou: $($args -join ' ')" }
}

if ($RemoveRoot) {
    $found = Get-ChildItem Cert:\CurrentUser\Root | Where-Object { $_.Subject -like "*CN=$caName*" }
    $found | Remove-Item
    Write-Host "Raízes de teste removidas: $(@($found).Count)"
    exit 0
}

if (-not (Test-Path $caCrt)) {
    New-Item -ItemType Directory -Force $dir | Out-Null
    $ext = Join-Path $dir 'ca.ext'
    Set-Content -Path $ext -Encoding ascii -Value @(
        '[req]', 'distinguished_name = dn', 'x509_extensions = v3_ca', 'prompt = no',
        '[dn]', "CN = $caName", 'O = Epistula Teste',
        '[v3_ca]', 'basicConstraints = critical, CA:true', 'keyUsage = critical, keyCertSign, cRLSign', 'subjectKeyIdentifier = hash'
    )
    Invoke-OpenSsl req -x509 -newkey rsa:3072 -noenc -keyout $caKey -out $caCrt -days 1825 -sha256 -config $ext
    Write-Host "CA de teste criada em $dir"
}

if ($InstallRoot) {
    # O Windows pede confirmação. Vale só para o seu usuário; -RemoveRoot desfaz.
    Import-Certificate -FilePath $caCrt -CertStoreLocation Cert:\CurrentUser\Root | Out-Null
    Write-Host 'CA de teste confiável no seu usuário do Windows. Feche e abra o Epistula para ele ler a raiz nova.'
}

function Sign-Request([string]$request, [string]$out) {
    $ext = Join-Path $dir 'leaf.ext'
    Set-Content -Path $ext -Encoding ascii -Value @('basicConstraints = CA:false', 'authorityKeyIdentifier = keyid', 'subjectKeyIdentifier = hash')
    # As extensões do pedido (e-mail, uso da chave, proteção de e-mail) vêm junto.
    Invoke-OpenSsl x509 -req -in $request -CA $caCrt -CAkey $caKey -CAcreateserial -days $Days -sha256 -copy_extensions copy -extfile $ext -out $out
}

if ($Csr) {
    $cert = [IO.Path]::ChangeExtension((Resolve-Path $Csr).Path, '.pem')
    Sign-Request (Resolve-Path $Csr).Path $cert
    # O certificado e a CA juntos: o Epistula guarda a CA como a cadeia.
    Get-Content $cert, $caCrt | Set-Content ([IO.Path]::ChangeExtension($cert, '.chain.pem')) -Encoding ascii
    Write-Host "Certificado emitido: $([IO.Path]::ChangeExtension($cert, '.chain.pem'))"
    Write-Host 'No Epistula: Configurações → Criptografia → Importar certificado.'
}

if ($Person) {
    $base = Join-Path (Get-Location) ($Person -replace '[^A-Za-z0-9@._-]', '-')
    $cnf = "$base.cnf"
    Set-Content -Path $cnf -Encoding ascii -Value @(
        '[req]', 'distinguished_name = dn', 'req_extensions = ext', 'prompt = no',
        '[dn]', "CN = $Name", "emailAddress = $Person",
        '[ext]', "subjectAltName = email:$Person", 'keyUsage = critical, digitalSignature, nonRepudiation, keyEncipherment', 'extendedKeyUsage = emailProtection'
    )
    Invoke-OpenSsl req -new -newkey rsa:3072 -noenc -keyout "$base.key" -out "$base.csr" -config $cnf
    Sign-Request "$base.csr" "$base.pem"
    $secure = Read-Host -AsSecureString 'Senha para o .p12 (8 ou mais caracteres)'
    $env:EPISTULA_P12_PASS = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
    try {
        Invoke-OpenSsl pkcs12 -export -inkey "$base.key" -in "$base.pem" -certfile $caCrt -name $Name -out "$base.p12" -passout env:EPISTULA_P12_PASS
    } finally {
        Remove-Item Env:\EPISTULA_P12_PASS
    }
    Remove-Item "$base.key", "$base.csr", $cnf
    Write-Host "Contato de teste: $base.p12 (com a chave) e $base.pem (só o certificado)"
}
