param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$Release,
    [switch]$SkipSigning,
    [switch]$SkipInstaller
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

if ($Target -ne "x86_64-pc-windows-msvc") {
    throw "Unsupported Windows package target '$Target'; expected x86_64-pc-windows-msvc"
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$MetadataJson = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $Root "Cargo.toml")
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed with exit code $LASTEXITCODE"
}
$Metadata = $MetadataJson | ConvertFrom-Json
$Package = $Metadata.packages | Where-Object { $_.name -eq "kerosene" } | Select-Object -First 1
if (!$Package) {
    throw "Could not find kerosene package metadata"
}
$Version = $Package.version
$ReleaseDir = Join-Path $Root "target\$Target\release"
$ExePath = Join-Path $ReleaseDir "kerosene.exe"
$DistDir = Join-Path $Root "target\windows-dist"
$IconPath = Join-Path $DistDir "kerosene.ico"
$ZipPath = Join-Path $DistDir "Kerosene-$Version-windows-x64.zip"
$MsiPath = Join-Path $DistDir "Kerosene-$Version-windows-x64.msi"
$ChecksumPath = Join-Path $DistDir "SHA256SUMS.txt"

function Write-Info([string]$Message) {
    Write-Host "[+] $Message"
}

function Assert-NativeSuccess([string]$Operation) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Operation failed with exit code $LASTEXITCODE"
    }
}

function New-IcoFromPng([string]$PngPath, [string]$OutputPath) {
    [byte[]]$Png = [System.IO.File]::ReadAllBytes($PngPath)
    $Stream = [System.IO.MemoryStream]::new()
    $Writer = [System.IO.BinaryWriter]::new($Stream)
    $Writer.Write([UInt16]0)
    $Writer.Write([UInt16]1)
    $Writer.Write([UInt16]1)
    $Writer.Write([byte]0)
    $Writer.Write([byte]0)
    $Writer.Write([byte]0)
    $Writer.Write([byte]0)
    $Writer.Write([UInt16]1)
    $Writer.Write([UInt16]32)
    $Writer.Write([UInt32]$Png.Length)
    $Writer.Write([UInt32]22)
    $Writer.Write($Png)
    [System.IO.File]::WriteAllBytes($OutputPath, $Stream.ToArray())
    $Writer.Dispose()
    $Stream.Dispose()
}

function Get-PeSubsystem([string]$Path) {
    [byte[]]$Bytes = [System.IO.File]::ReadAllBytes($Path)
    $PeOffset = [BitConverter]::ToInt32($Bytes, 0x3c)
    $OptionalHeaderOffset = $PeOffset + 24
    return [BitConverter]::ToUInt16($Bytes, $OptionalHeaderOffset + 68)
}

function Get-PeSectionNames([string]$Path) {
    [byte[]]$Bytes = [System.IO.File]::ReadAllBytes($Path)
    $PeOffset = [BitConverter]::ToInt32($Bytes, 0x3c)
    $SectionCount = [BitConverter]::ToUInt16($Bytes, $PeOffset + 6)
    $OptionalHeaderSize = [BitConverter]::ToUInt16($Bytes, $PeOffset + 20)
    $SectionTableOffset = $PeOffset + 24 + $OptionalHeaderSize

    for ($Index = 0; $Index -lt $SectionCount; $Index++) {
        $NameOffset = $SectionTableOffset + ($Index * 40)
        $RawName = $Bytes[$NameOffset..($NameOffset + 7)]
        [System.Text.Encoding]::ASCII.GetString($RawName).Trim([char]0)
    }
}

function Assert-WindowsGuiExecutable([string]$Path) {
    $Subsystem = Get-PeSubsystem $Path
    if ($Subsystem -ne 2) {
        throw "Expected Windows GUI subsystem (2), found subsystem $Subsystem"
    }

    $Sections = @(Get-PeSectionNames $Path)
    if (!($Sections -contains ".rsrc")) {
        throw "Expected Windows resource section (.rsrc) in $Path"
    }
}

function Invoke-SignFile([string]$Path) {
    if ($Release -and $SkipSigning) {
        throw "Release builds cannot skip Windows signing"
    }
    if ($SkipSigning) {
        Write-Info "Skipping signing for $Path"
        return
    }

    $PfxBase64 = $env:WINDOWS_SIGNING_CERT_PFX_BASE64
    $PfxPassword = $env:WINDOWS_SIGNING_CERT_PASSWORD
    if ([string]::IsNullOrWhiteSpace($PfxBase64) -or [string]::IsNullOrWhiteSpace($PfxPassword)) {
        if ($Release) {
            throw "Windows signing secrets are required for release builds"
        }
        Write-Info "Signing secrets are unavailable; leaving $Path unsigned"
        return
    }

    $SignTool = Get-SignTool
    $TempRoot = $env:RUNNER_TEMP
    if ([string]::IsNullOrWhiteSpace($TempRoot)) {
        $TempRoot = [System.IO.Path]::GetTempPath()
    }
    $PfxPath = Join-Path $TempRoot ("kerosene-signing-{0}.pfx" -f [Guid]::NewGuid().ToString("N"))
    [System.IO.File]::WriteAllBytes($PfxPath, [Convert]::FromBase64String($PfxBase64))
    try {
        & $SignTool sign /f $PfxPath /p $PfxPassword /fd SHA256 /tr "http://timestamp.digicert.com" /td SHA256 $Path
        Assert-NativeSuccess "Authenticode signing for $Path"
        & $SignTool verify /pa /v $Path
        Assert-NativeSuccess "Authenticode verification for $Path"
    }
    finally {
        if (Test-Path $PfxPath) {
            Remove-Item $PfxPath -Force
        }
    }
}

function Get-SignTool {
    $Command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($Command) {
        return $Command.Source
    }

    $ProgramFilesX86 = ${env:ProgramFiles(x86)}
    if ([string]::IsNullOrWhiteSpace($ProgramFilesX86)) {
        throw "ProgramFiles(x86) is unavailable; signtool.exe was not found on PATH."
    }
    $KitsRoot = Join-Path $ProgramFilesX86 "Windows Kits\10\bin"
    $Candidates = Get-ChildItem -Path $KitsRoot -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\x64\\signtool.exe$" } |
        Sort-Object FullName -Descending
    if ($Candidates) {
        return $Candidates[0].FullName
    }

    throw "signtool.exe was not found. Install the Windows SDK or run from a Visual Studio developer shell."
}

function Write-Checksums([string[]]$Paths, [string]$OutputPath) {
    $Lines = foreach ($Path in $Paths) {
        $Hash = Get-FileHash -Algorithm SHA256 $Path
        "$($Hash.Hash.ToLowerInvariant())  $(Split-Path $Path -Leaf)"
    }
    $Lines | Set-Content -Encoding ascii $OutputPath
}

New-Item -ItemType Directory -Force $DistDir | Out-Null
New-IcoFromPng (Join-Path $Root "assets\kerosene.png") $IconPath

Write-Info "Installing Rust target $Target if needed"
rustup target add $Target
Assert-NativeSuccess "rustup target installation"

Write-Info "Building Windows release binary"
if (Test-Path $ExePath) {
    Remove-Item $ExePath -Force
}
cargo build --release --target $Target --manifest-path (Join-Path $Root "Cargo.toml")
Assert-NativeSuccess "cargo Windows release build"

if (!(Test-Path $ExePath)) {
    throw "Missing Windows executable at $ExePath"
}

Assert-WindowsGuiExecutable $ExePath
Invoke-SignFile $ExePath

if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
}
Write-Info "Creating portable zip"
Compress-Archive -Path $ExePath -DestinationPath $ZipPath

if (!$SkipInstaller) {
    $Wix = Get-Command wix.exe -ErrorAction SilentlyContinue
    if (!$Wix) {
        throw "WiX CLI is required to build the Windows installer. Install with: dotnet tool install --global wix"
    }
    if (Test-Path $MsiPath) {
        Remove-Item $MsiPath -Force
    }
    Write-Info "Building MSI installer"
    & $Wix.Source build `
        (Join-Path $Root "packaging\windows\Kerosene.wxs") `
        -arch x64 `
        -d "ProductVersion=$Version" `
        -d "SourceDir=$ReleaseDir" `
        -d "IconPath=$IconPath" `
        -out $MsiPath
    Assert-NativeSuccess "WiX MSI build"
    if (!(Test-Path $MsiPath)) {
        throw "WiX reported success but did not create $MsiPath"
    }
    Invoke-SignFile $MsiPath
}

$Artifacts = @($ZipPath)
if (Test-Path $MsiPath) {
    $Artifacts += $MsiPath
}
Write-Checksums $Artifacts $ChecksumPath

Write-Info "Windows artifacts written to $DistDir"
