// Simple script to generate placeholder icons for Tauri
// This creates minimal valid image files for development

const fs = require("fs");
const path = require("path");

const iconsDir = path.join(__dirname, "src-tauri", "icons");

// Create icons directory
if (!fs.existsSync(iconsDir)) {
  fs.mkdirSync(iconsDir, { recursive: true });
}

// Minimal 1x1 PNG (base64 encoded)
const minimalPNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==", "base64");

// Minimal ICO file (16x16, 1-bit)
const minimalICO = Buffer.from([0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x10, 0x10, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0x68, 0x04, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, ...Array(1024).fill(0x00), 0x00, 0x00, 0x00, 0x00]);

// Minimal ICNS header
const minimalICNS = Buffer.from("icns\x00\x00\x00\x08", "binary");

// Generate PNG files
["32x32.png", "128x128.png", "128x128@2x.png"].forEach((filename) => {
  fs.writeFileSync(path.join(iconsDir, filename), minimalPNG);
  console.log(`✓ Created ${filename}`);
});

// Generate ICO file
fs.writeFileSync(path.join(iconsDir, "icon.ico"), minimalICO);
console.log("✓ Created icon.ico");

// Generate ICNS file
fs.writeFileSync(path.join(iconsDir, "icon.icns"), minimalICNS);
console.log("✓ Created icon.icns");

console.log("\n✅ Placeholder icons generated successfully!");
console.log("📝 Note: Replace these with proper icons before production release.");
