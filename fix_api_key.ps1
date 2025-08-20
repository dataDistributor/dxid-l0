# Fix API Key Script
# This script helps you set the correct API key in the CLI

Write-Host "=== dxID Layer0 API Key Fix ===" -ForegroundColor Green

# Check if admin token exists
if (Test-Path "dxid-data\admin_token.txt") {
    $adminToken = Get-Content "dxid-data\admin_token.txt" -Raw
    Write-Host "Admin token found: $($adminToken.Trim())" -ForegroundColor Yellow
} else {
    Write-Host "Admin token not found!" -ForegroundColor Red
    exit 1
}

# Check if API key was created
Write-Host "`nChecking for API keys..." -ForegroundColor Cyan

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8545/admin/apikeys" -Headers @{"X-Admin-Token" = $adminToken.Trim()} -Method Get
    Write-Host "API Keys found:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Failed to get API keys: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

Write-Host "`nTo set the correct API key in the CLI:" -ForegroundColor Yellow
Write-Host "1. Run the CLI: cargo run --bin dxid-cli-enhanced" -ForegroundColor White
Write-Host "2. Choose option 5 (API key management)" -ForegroundColor White
Write-Host "3. Choose option 2 (Set API key)" -ForegroundColor White
Write-Host "4. Enter the 'secret' value from above (not the name!)" -ForegroundColor White
Write-Host "`nExample: If the secret is 'f87966b448a1686509d1ee1b7c57ff5b823b12e95254f8e164398b13bbb77402', enter that." -ForegroundColor Cyan
