# Installs Shape for this Windows user (no admin rights needed):
#
#   irm https://dobrinov.github.io/shape/install.ps1 | iex
#
# shape.exe goes in %LOCALAPPDATA%\Programs\Shape and on your PATH, Shape goes
# in the Start menu, and .shape files open in it with a double-click. Run it
# again to upgrade. To remove: shape install-app --remove, then delete that
# folder.
& {
  $ErrorActionPreference = 'Stop'
  $ProgressPreference = 'SilentlyContinue' # the progress bar makes downloads slow
  $repo = 'dobrinov/shape'
  if (-not [Environment]::Is64BitOperatingSystem) { throw 'Shape needs 64-bit Windows.' }
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

  $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'shape-install' }
  $version = $release.tag_name.TrimStart('v')
  $name = "shape-$version-windows-x86_64.zip"
  $base = "https://github.com/$repo/releases/download/v$version"

  $tmp = Join-Path ([IO.Path]::GetTempPath()) "shape-install-$version"
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  New-Item -ItemType Directory $tmp | Out-Null
  Write-Host "Downloading Shape $version..."
  try {
    Invoke-WebRequest "$base/$name" -OutFile "$tmp\$name" -UseBasicParsing
    Invoke-WebRequest "$base/SHA256SUMS" -OutFile "$tmp\SHA256SUMS" -UseBasicParsing
  } catch {
    throw "Shape $version for Windows is not out yet (the release is still being built?). Try again in a few minutes."
  }
  $want = (Get-Content "$tmp\SHA256SUMS" | Where-Object { $_ -match [regex]::Escape($name) }) -split '\s+' | Select-Object -First 1
  $got = (Get-FileHash "$tmp\$name" -Algorithm SHA256).Hash
  if (-not $want -or $got -ne $want.ToUpper()) { throw "The download is damaged (its checksum does not match). Try again." }
  Expand-Archive "$tmp\$name" -DestinationPath "$tmp\files" -Force

  $dir = Join-Path $env:LOCALAPPDATA 'Programs\Shape'
  New-Item -ItemType Directory -Force $dir | Out-Null
  try {
    Copy-Item "$tmp\files\*" $dir -Force
  } catch {
    throw "Could not replace $dir\shape.exe: close Shape (every board's window) and run this again."
  }
  # before 0.73 the alias was a second copy of shape.exe
  Remove-Item "$dir\diagram.exe" -ErrorAction SilentlyContinue
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue

  # on PATH, for new terminals and this one
  $path = [Environment]::GetEnvironmentVariable('Path', 'User')
  $parts = @($path -split ';' | Where-Object { $_ })
  if ($parts -notcontains $dir) {
    [Environment]::SetEnvironmentVariable('Path', (($parts + $dir) -join ';'), 'User')
  }
  if (($env:Path -split ';') -notcontains $dir) { $env:Path = "$env:Path;$dir" }

  # .shape files and the Start menu
  & "$dir\shape.exe" install-app
  Write-Host ""
  Write-Host "Shape $version is installed. Open it from the Start menu, double-click a .shape file, or type shape in a terminal."
}
