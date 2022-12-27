#!/usr/bin/env pwsh
rustup show
if ( ( rustc --version ) -match 'rustc ([0-9]+\.[0-9]+\.[0-9]+[-a-z]*) \(([0-9a-f]+)\ ' ) {
    'value='+$Matches[1]+'-'+$Matches[2] | Out-File $env:GITHUB_OUTPUT
} else {
    exit 1
}
