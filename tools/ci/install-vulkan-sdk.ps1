# Installs the pinned Vulkan SDK and the Vulkan loader on a CI runner.
# whisper.cpp's Vulkan backend needs the SDK to BUILD (vulkan-1.lib, the headers, and glslc to
# compile its shaders), and every executable that links it imports vulkan-1.dll, so a test binary
# does not even START without the loader. On a user's machine the GPU driver puts that DLL into
# System32; a CI runner has no GPU driver, so the loader comes from LunarG's runtime components
# archive and goes on PATH.
# Both downloads are pinned by version AND by SHA-256. LunarG publishes the hashes at
# https://sdk.lunarg.com/sdk/sha/<version>/windows/<file>.txt : bump all three together, and keep
# the version equal to the one src-tauri/.cargo/config.toml names.
# The installer runs unattended, which accepts the SDK's licence on the runner. A developer
# machine does not need this script: `winget install KhronosGroup.VulkanSDK` is the README's way.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$version   = "1.4.357.0"
$sdkName   = "vulkansdk-windows-X64-$version.exe"
$sdkSha256 = "81f474711e9042f4cd22b31b2f7a8870db2e428b21586fb43dd80150be97310d"
$rtName    = "VulkanRT-X64-$version-Components.zip"
$rtSha256  = "a14672efed15aafc7f5a16572d35cd3a3416eadf670aeee3cdf50ee32d5fbf83"
$base      = "https://sdk.lunarg.com/sdk/download/$version/windows"

$dest    = "C:\VulkanSDK\$version"
$runtime = Join-Path $dest "runtime"
$tmp     = Join-Path ([IO.Path]::GetTempPath()) "tcursor-vulkan"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

function Get-Pinned([string]$Name, [string]$Sha256) {
  $file = Join-Path $tmp $Name
  Write-Host "Downloading $base/$Name"
  Invoke-WebRequest -Uri "$base/$Name" -OutFile $file
  $actual = (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $Sha256) { throw "$Name checksum mismatch: expected $Sha256, got $actual" }
  return $file
}

if (Test-Path (Join-Path $dest "Lib\vulkan-1.lib")) {
  Write-Host "The Vulkan SDK is already in $dest"
} else {
  $installer = Get-Pinned $sdkName $sdkSha256
  $run = Start-Process -FilePath $installer -Wait -PassThru -ArgumentList @(
    "--root", $dest, "--accept-licenses", "--default-answer", "--confirm-command", "install")
  if ($run.ExitCode -ne 0) { throw "The Vulkan SDK installer exited with code $($run.ExitCode)" }
}

$system = Join-Path $env:WINDIR "System32\vulkan-1.dll"
if (Test-Path $system) {
  Write-Host "The Vulkan loader is already in System32"
} elseif (-not (Test-Path (Join-Path $runtime "vulkan-1.dll"))) {
  $zip = Get-Pinned $rtName $rtSha256
  $out = Join-Path $tmp "runtime"
  Expand-Archive -Path $zip -DestinationPath $out -Force
  $dll = Get-ChildItem $out -Recurse -Filter "vulkan-1.dll" |
    Where-Object { $_.DirectoryName -match "x64$" } | Select-Object -First 1
  if (-not $dll) { throw "$rtName has no x64 vulkan-1.dll" }
  New-Item -ItemType Directory -Force -Path $runtime | Out-Null
  Copy-Item $dll.FullName $runtime -Force
}

$needed = @("Lib\vulkan-1.lib", "Include\vulkan\vulkan.h", "Bin\glslc.exe")
if (-not (Test-Path $system)) { $needed += "runtime\vulkan-1.dll" }
foreach ($need in $needed) {
  if (-not (Test-Path (Join-Path $dest $need))) { throw "The Vulkan SDK at $dest is missing $need" }
}

if ($env:GITHUB_ENV)  { Add-Content -Path $env:GITHUB_ENV  -Value "VULKAN_SDK=$dest" }
if ($env:GITHUB_PATH) { Add-Content -Path $env:GITHUB_PATH -Value (Join-Path $dest "Bin"), $runtime }
Write-Host "Vulkan SDK $version is ready in $dest"
