param(
    [ValidateSet("windows-x64", "windows-arm64")]
    [string]$Platform = "windows-x64"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Version = (Get-Content (Join-Path $Root "packaging\pi\version.txt") -Raw).Trim()
$ChecksumLines = Get-Content (Join-Path $Root "packaging\pi\SHA256SUMS")
$ArchiveName = "pi-$Platform.zip"
$ChecksumLine = $ChecksumLines | Where-Object { $_ -match "\s+$([regex]::Escape($ArchiveName))$" } | Select-Object -First 1
if (!$ChecksumLine) {
    throw "No pinned checksum exists for $ArchiveName"
}
$ExpectedSha = ($ChecksumLine -split "\s+")[0].ToLowerInvariant()

$CacheDir = Join-Path $Root "target\pi\$Version\$Platform"
$BundlePath = Join-Path $CacheDir "bundle"
$BinaryPath = Join-Path $BundlePath "pi.exe"
$DownloadDir = Join-Path $Root "target\pi\downloads\$Version"
$ArchivePath = Join-Path $DownloadDir $ArchiveName

function Test-PiBinary([string]$Path) {
    if (!(Test-Path $Path -PathType Leaf)) {
        return $false
    }
    try {
        $Reported = (& $Path --version | Select-Object -First 1)
        return $Reported -and $Reported.Contains($Version)
    }
    catch {
        return $false
    }
}

function Test-PiBundle([string]$Path) {
    return (Test-PiBinary (Join-Path $Path "pi.exe")) `
        -and (Test-Path (Join-Path $Path "package.json") -PathType Leaf) `
        -and (Test-Path (Join-Path $Path "theme\dark.json") -PathType Leaf) `
        -and (Test-Path (Join-Path $Path "theme\light.json") -PathType Leaf)
}

function Test-PiRpc([string]$Path) {
    $SmokeDir = Join-Path (Join-Path $Root "target\pi") ("smoke-" + [Guid]::NewGuid().ToString("N"))
    $PreviousConfigDir = $env:PI_CODING_AGENT_DIR
    $PreviousSkipVersionCheck = $env:PI_SKIP_VERSION_CHECK
    $PreviousTelemetry = $env:PI_TELEMETRY
    $PreviousOpenRouterKey = $env:OPENROUTER_API_KEY
    $PreviousHyperDashKey = $env:KEROSENE_AGENT_HYPERDASH_API_KEY
    $PreviousSnapshot = $env:KEROSENE_AGENT_SNAPSHOT
    New-Item -ItemType Directory $SmokeDir | Out-Null
    try {
        $env:PI_CODING_AGENT_DIR = Join-Path $SmokeDir "config"
        $env:PI_SKIP_VERSION_CHECK = "1"
        $env:PI_TELEMETRY = "0"
        $env:OPENROUTER_API_KEY = "rpc-smoke-test"
        $env:KEROSENE_AGENT_HYPERDASH_API_KEY = ""
        $env:KEROSENE_AGENT_SNAPSHOT = Join-Path $SmokeDir "snapshot.json"
        Push-Location $SmokeDir
        try {
            $Response = '{"type":"get_state"}' | & (Join-Path $Path "pi.exe") `
                --mode rpc `
                --no-session `
                --provider openrouter `
                --model openai/gpt-4.1 `
                --tools kerosene_data `
                --extension (Join-Path $Root "assets\agent\kerosene.ts") 2>&1
        }
        finally {
            Pop-Location
        }
        return [bool]($Response | Where-Object { $_ -like '*"command":"get_state","success":true*' })
    }
    catch {
        return $false
    }
    finally {
        $env:PI_CODING_AGENT_DIR = $PreviousConfigDir
        $env:PI_SKIP_VERSION_CHECK = $PreviousSkipVersionCheck
        $env:PI_TELEMETRY = $PreviousTelemetry
        $env:OPENROUTER_API_KEY = $PreviousOpenRouterKey
        $env:KEROSENE_AGENT_HYPERDASH_API_KEY = $PreviousHyperDashKey
        $env:KEROSENE_AGENT_SNAPSHOT = $PreviousSnapshot
        if (Test-Path $SmokeDir) {
            Remove-Item $SmokeDir -Recurse -Force
        }
    }
}

if ((Test-PiBundle $BundlePath) -and (Test-PiRpc $BundlePath)) {
    Write-Output $BundlePath
    exit 0
}

New-Item -ItemType Directory -Force $CacheDir, $DownloadDir | Out-Null
if (Test-Path $ArchivePath -PathType Leaf) {
    $ActualSha = (Get-FileHash -Algorithm SHA256 $ArchivePath).Hash.ToLowerInvariant()
    if ($ActualSha -ne $ExpectedSha) {
        Write-Host "[!] Discarding cached $ArchiveName with an invalid checksum"
        Remove-Item $ArchivePath -Force
    }
}

if (!(Test-Path $ArchivePath -PathType Leaf)) {
    Write-Host "[+] Downloading Pi $Version for $Platform"
    $TemporaryArchive = "$ArchivePath.download"
    if (Test-Path $TemporaryArchive) {
        Remove-Item $TemporaryArchive -Force
    }
    Invoke-WebRequest `
        -Uri "https://github.com/earendil-works/pi/releases/download/v$Version/$ArchiveName" `
        -OutFile $TemporaryArchive
    $ActualSha = (Get-FileHash -Algorithm SHA256 $TemporaryArchive).Hash.ToLowerInvariant()
    if ($ActualSha -ne $ExpectedSha) {
        Remove-Item $TemporaryArchive -Force
        throw "Checksum verification failed for $ArchiveName"
    }
    Move-Item $TemporaryArchive $ArchivePath
}

$TemporaryDir = Join-Path (Join-Path $Root "target\pi") ("extract-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $TemporaryDir | Out-Null
try {
    Expand-Archive -Path $ArchivePath -DestinationPath $TemporaryDir
    $SourceBinary = Get-ChildItem -Path $TemporaryDir -Filter pi.exe -File -Recurse | Select-Object -First 1
    if (!$SourceBinary) {
        throw "$ArchiveName does not contain a Pi executable"
    }
    $SourceDir = $SourceBinary.Directory.FullName
    $StagedBundle = Join-Path $TemporaryDir "runtime-bundle"
    New-Item -ItemType Directory (Join-Path $StagedBundle "theme") | Out-Null
    Copy-Item $SourceBinary.FullName (Join-Path $StagedBundle "pi.exe")
    Copy-Item (Join-Path $SourceDir "package.json") (Join-Path $StagedBundle "package.json")
    Copy-Item (Join-Path $SourceDir "theme\dark.json") (Join-Path $StagedBundle "theme\dark.json")
    Copy-Item (Join-Path $SourceDir "theme\light.json") (Join-Path $StagedBundle "theme\light.json")
    Copy-Item (Join-Path $SourceDir "theme\theme-schema.json") (Join-Path $StagedBundle "theme\theme-schema.json")
    if (Test-Path $BundlePath) {
        Remove-Item $BundlePath -Recurse -Force
    }
    Move-Item $StagedBundle $BundlePath
}
finally {
    if (Test-Path $TemporaryDir) {
        Remove-Item $TemporaryDir -Recurse -Force
    }
}

if (!(Test-PiBundle $BundlePath)) {
    Remove-Item $BundlePath -Recurse -Force -ErrorAction SilentlyContinue
    throw "The extracted Pi bundle did not report the pinned version $Version"
}
if (!(Test-PiRpc $BundlePath)) {
    Remove-Item $BundlePath -Recurse -Force -ErrorAction SilentlyContinue
    throw "Pi failed the offline RPC startup smoke test"
}

Write-Output $BundlePath
