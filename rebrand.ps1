$ErrorActionPreference = 'Stop'
Set-Location 'D:\Yappuccino'

function New-Name([string]$value) {
    return [regex]::Replace($value, '(?i)yapfast', {
        param($m)
        switch -CaseSensitive ($m.Value) {
            'YAPFAST' { 'YAPPUCCINO' }
            'Yapfast' { 'Yappuccino' }
            default { 'yappuccino' }
        }
    })
}

# 1. Rename every tracked path that carries the old name.
$paths = @(git ls-files) | Where-Object { $_ -match '(?i)yapfast' }
foreach ($old in $paths) {
    $new = New-Name $old
    $parent = Split-Path -Parent $new
    if ($parent -and -not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    if (Test-Path -LiteralPath $new) { throw "destination exists: $new" }
    git mv -- $old $new
    if ($LASTEXITCODE -ne 0) { throw "git mv failed: $old -> $new" }
}
Write-Host "renamed paths: $($paths.Count)"

# 2. Replace the name in text files.
$utf8 = New-Object System.Text.UTF8Encoding($false)
$changed = 0
foreach ($file in @(git ls-files)) {
    $full = Join-Path (Get-Location) $file
    try { $bytes = [System.IO.File]::ReadAllBytes($full) } catch { continue }
    $probe = [Math]::Min($bytes.Length, 8192)
    $binary = $false
    for ($i = 0; $i -lt $probe; $i++) { if ($bytes[$i] -eq 0) { $binary = $true; break } }
    if ($binary) { continue }
    try {
        $text = $utf8.GetString($bytes)
        $decoded = New-Object System.Text.UTF8Encoding($false, $true)
        $null = $decoded.GetString($bytes)
    } catch { continue }
    if ($text -notmatch '(?i)yapfast') { continue }
    $updated = New-Name $text
    if ($updated -ne $text) {
        [System.IO.File]::WriteAllText($full, $updated, $utf8)
        $changed++
    }
}
Write-Host "rewritten files: $changed"

# 3. Keep links to the real upstream project working: the repository keeps
#    its own name, while the product is Yappuccino.
$fixed = 0
foreach ($file in @(git ls-files)) {
    $full = Join-Path (Get-Location) $file
    try { $bytes = [System.IO.File]::ReadAllBytes($full) } catch { continue }
    $probe = [Math]::Min($bytes.Length, 8192)
    $binary = $false
    for ($i = 0; $i -lt $probe; $i++) { if ($bytes[$i] -eq 0) { $binary = $true; break } }
    if ($binary) { continue }
    try {
        $text = $utf8.GetString($bytes)
        $decoded = New-Object System.Text.UTF8Encoding($false, $true)
        $null = $decoded.GetString($bytes)
    } catch { continue }
    $updated = $text
    $updated = $updated.Replace('https://github.com/yappuccino/yappuccino', 'https://github.com/Dev-NotAqua/yapfast')
    $updated = $updated.Replace('https://img.shields.io/github/license/yappuccino/yappuccino', 'https://img.shields.io/github/license/Dev-NotAqua/yapfast')
    $updated = $updated.Replace('https://img.shields.io/github/v/release/yappuccino/yappuccino', 'https://img.shields.io/github/v/release/Dev-NotAqua/yapfast')
    $updated = $updated.Replace('https://raw.githubusercontent.com/yappuccino/yappuccino', 'https://raw.githubusercontent.com/Dev-NotAqua/yapfast')
    $updated = $updated.Replace('https://discord.gg/yappuccino', 'https://discord.gg/yapfast')
    $updated = $updated.Replace('https://discord.com/invite/yappuccino', 'https://discord.com/invite/yapfast')
    $updated = $updated.Replace('https://yappuccino.com', 'https://yapfast.com')
    if ($updated -ne $text) {
        [System.IO.File]::WriteAllText($full, $updated, $utf8)
        $fixed++
    }
}
Write-Host "link fixes: $fixed"

# 4. Remove directories left empty by the moves.
$dirs = Get-ChildItem -Directory -Recurse -Force | Where-Object { $_.FullName -notmatch '\\\.git(\\|$)' } | Sort-Object { $_.FullName.Length } -Descending
foreach ($dir in $dirs) {
    if (-not (Get-ChildItem -LiteralPath $dir.FullName -Force | Select-Object -First 1)) {
        Remove-Item -LiteralPath $dir.FullName -Force
    }
}
Write-Host 'done'
