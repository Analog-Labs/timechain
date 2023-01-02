#!/usr/bin/env pwsh

if ( $env:GITHUB_OUTPUT -ne '' ) {
    Remove-Item -Force ~/.cargo/bin/cargo-fmt -ea 0
    Remove-Item -Force ~/.cargo/bin/rustfmt -ea 0
}

$defult_nightly = (
    Get-Content .\.github\config.txt |
    Select-String -Pattern 'toolchain\s*=\s*([a-z0-9-]+)' |
    % { $_.Matches.Groups[1].Value }) ?? 'nightly'
$nightly = $env:WASM_BUILD_TOOLCHAIN ?? $defult_nightly

rustup toolchain install $nightly -c rustfmt,clippy
if (!$?) { exit $? }
rustup target add wasm32-unknown-unknown --toolchain $nightly
if (!$?) { exit $? }
rustup show
if (!$?) { exit $? }

$a = rustc --version |
        Select-String -Pattern 'rustc [0-9]+\.([0-9]+)\.[0-9]+-?([a-z]*) \(([0-9a-f]+)\ ' |
        % { $_.Matches.Groups } |
        Where Name -ge 1 |
        Join-String -Separator _
if (!$?) { exit $? }

$b = rustc +$nightly --version |
        Select-String -Pattern 'rustc [0-9]+\.([0-9]+)\.[0-9]+-?([a-z]*) \(([0-9a-f]+)\ ' |
        % { $_.Matches.Groups } |
        Where Name -ge 1 |
        Join-String -Separator _
if (!$?) { exit $? }

switch ( $env:GITHUB_OUTPUT )
{
    '' {
        "value=${a}+${b}" | Write-Output
    }
    default {
        "value=${a}+${b}" | Out-File $env:GITHUB_OUTPUT
    }
}

rustup override set $nightly
