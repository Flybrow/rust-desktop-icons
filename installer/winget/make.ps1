param([Parameter(Mandatory)][string]$Version, [string]$Out = "target/winget")
$ErrorActionPreference = 'Stop'
$id = 'Flybrow.RustDesktopIcons'
$repo = 'https://github.com/Flybrow/rust-desktop-icons'
$msi = "rust-desktop-icons-$Version-x64.msi"
$url = "$repo/releases/download/v$Version/$msi"
$date = ([datetime](Invoke-RestMethod "https://api.github.com/repos/Flybrow/rust-desktop-icons/releases/tags/v$Version").published_at).ToString('yyyy-MM-dd')
$sha = ((Invoke-RestMethod "$url.sha256") -split '\s+')[0].ToUpper()
if ($sha -notmatch '^[0-9A-F]{64}$') { throw "bad checksum for $msi" }
$dir = Join-Path $Out "$Version"
New-Item -ItemType Directory -Force $dir | Out-Null
$schema = '1.6.0'
$head = { param($kind) "# yaml-language-server: `$schema=https://aka.ms/winget-manifest.$kind.$schema.schema.json`n" }
$common = "PackageIdentifier: $id`nPackageVersion: $Version`n"
$files = @{
  "$id.yaml" = (& $head 'version') + $common + "DefaultLocale: en-US`nManifestType: version`nManifestVersion: $schema`n"
  "$id.installer.yaml" = (& $head 'installer') + $common + @"
InstallerType: wix
Scope: user
InstallModes:
- interactive
- silent
UpgradeBehavior: install
ReleaseDate: $date
AppsAndFeaturesEntries:
- DisplayName: Rust Desktop Icons
  Publisher: Flybrow
  UpgradeCode: '{EC06B7E9-B60A-4EFC-9A07-9FC0C215C3B2}'
Installers:
- Architecture: x64
  InstallerUrl: $url
  InstallerSha256: $sha
ManifestType: installer
ManifestVersion: $schema

"@
  "$id.locale.en-US.yaml" = (& $head 'defaultLocale') + $common + @"
PackageLocale: en-US
Publisher: Flybrow
PublisherUrl: https://github.com/Flybrow
PublisherSupportUrl: $repo/issues
PackageName: Rust Desktop Icons
PackageUrl: $repo
License: MIT
LicenseUrl: $repo/blob/main/LICENSE
ShortDescription: Free, open-source Fences alternative that groups desktop icons into translucent or frosted-glass fences.
Description: Organize your Windows desktop into fences with tabs, sorting rules, roll-up, peek and search. Tiny, fast and private, written in Rust.
Moniker: rust-desktop-icons
Tags:
- desktop
- desktop-icons
- desktop-organizer
- fences
- fences-alternative
- productivity
- windows-desktop
ReleaseNotesUrl: $repo/releases/tag/v$Version
ManifestType: defaultLocale
ManifestVersion: $schema

"@
  "$id.locale.fr-FR.yaml" = (& $head 'locale') + $common + @"
PackageLocale: fr-FR
Publisher: Flybrow
PackageName: Rust Desktop Icons
License: MIT
ShortDescription: Alternative gratuite et open source à Fences qui range les icônes du bureau dans des zones translucides ou en verre dépoli.
Description: Rangez votre bureau Windows dans des zones avec onglets, règles de tri, repli, aperçu et recherche. Légère, rapide et respectueuse de la vie privée, écrite en Rust.
ManifestType: locale
ManifestVersion: $schema

"@
}
foreach ($f in $files.Keys) { [IO.File]::WriteAllText((Join-Path $dir $f), $files[$f], [Text.UTF8Encoding]::new($false)) }
Write-Output $dir
if (Get-Command winget -ErrorAction SilentlyContinue) { winget validate --manifest $dir }
