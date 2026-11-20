param([string] $EasyPdfFontsRoot)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'font-notices.ps1')
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('easypdf-font-notices-' + [guid]::NewGuid())
function Assert-Rejected([scriptblock] $Action, [string] $Message) {
  $rejected = $false
  try { & $Action | Out-Null } catch { $rejected = $true }
  if (-not $rejected) { throw "Expected rejection: $Message" }
}
try {
  foreach ($family in 'core', 'chinese') {
    New-Item -ItemType Directory -Path (Join-Path $fixture "fonts/$family") -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixture "fonts/$family/OFL.txt") -Value 'Synthetic notice fixture'
  }
  $core = 'Core | Regular | fonts/core/face.ttf | Latn | OFL-1.1 | fonts/core/OFL.txt'
  $chinese = 'Chinese | Regular | fonts/chinese/face.ttf | Hans | OFL-1.1 | fonts/chinese/OFL.txt'
  Set-Content (Join-Path $fixture 'fonts/manifest.txt') @('# comment', $core, $core, $chinese)
  if (@(Get-EasyPdfFontNotices $fixture).Count -ne 2) { throw 'Expected both unique family notices' }
  Set-Content (Join-Path $fixture 'fonts/manifest.txt') @($core, $chinese.Replace('fonts/chinese/OFL.txt', 'fonts/core/OFL.txt'))
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'another family cannot supply this notice'
  Set-Content (Join-Path $fixture 'fonts/manifest.txt') @($core, $chinese)
  Remove-Item (Join-Path $fixture 'fonts/chinese/OFL.txt')
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'one remaining OFL cannot cover the Chinese family'
  Set-Content (Join-Path $fixture 'fonts/chinese/OFL.txt') '' -NoNewline
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'empty family notice'
  Set-Content (Join-Path $fixture 'fonts/manifest.txt') @($core, $chinese.Replace('fonts/chinese/OFL.txt', '../OFL.txt'))
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'notice path escapes font bundle'
  Remove-Item (Join-Path $fixture 'fonts/manifest.txt')
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'main manifest missing'
  Set-Content (Join-Path $fixture 'fonts/manifest.txt') 'malformed'
  Assert-Rejected { Get-EasyPdfFontNotices $fixture } 'malformed metadata'
  Assert-EasyPdfFontNoticeNames @('OFL-core.txt', 'OFL-chinese.txt') @('SHORT.TXT|OFL-core.txt', 'OFL-chinese.txt')
  Assert-Rejected { Assert-EasyPdfFontNoticeNames @('OFL-core.txt', 'OFL-chinese.txt') @('OFL-core.txt') } 'MSI lost one staged notice'
  if ($EasyPdfFontsRoot) {
    $actual = @(Get-EasyPdfFontNotices $EasyPdfFontsRoot)
    Write-Output "Verified $($actual.Count) notices from the internal easyPDF manifest"
  }
  Write-Output 'Font notice regression checks passed'
} finally {
  Remove-Item -LiteralPath $fixture -Recurse -Force
}
