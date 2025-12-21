#!/usr/bin/env python3
"""Generate a simple 1024x1024 PNG icon for Tauri"""

import struct
import zlib

def create_png(width, height, filename):
    """Create a minimal PNG file"""
    # Create RGBA image data (blue gradient with white magnifying glass icon)
    data = bytearray()
    for y in range(height):
        data.append(0)  # Filter type
        for x in range(width):
            # Simple blue gradient background
            r, g, b, a = 100, 108, 255, 255
            
            # Draw a simple magnifying glass icon in the center
            cx, cy = width // 2, height // 2
            dx, dy = x - cx, y - cy
            dist = (dx*dx + dy*dy) ** 0.5
            
            # Circle for lens (radius 200)
            if 180 < dist < 220:
                r, g, b, a = 255, 255, 255, 255
            elif dist < 180:
                r, g, b, a = 255, 255, 255, 200
            
            # Handle for magnifying glass
            if 220 < dist < 380 and -45 < (dy/dx if dx != 0 else 999) * 180 / 3.14159 < -35:
                r, g, b, a = 255, 255, 255, 255
            
            data.extend([r, g, b, a])
    
    compressed = zlib.compress(bytes(data), 9)
    
    def chunk(type, data):
        length = struct.pack('>I', len(data))
        crc = zlib.crc32(type + data) & 0xffffffff
        return length + type + data + struct.pack('>I', crc)
    
    ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    
    png = b'\x89PNG\r\n\x1a\n'
    png += chunk(b'IHDR', ihdr)
    png += chunk(b'IDAT', compressed)
    png += chunk(b'IEND', b'')
    
    with open(filename, 'wb') as f:
        f.write(png)
    
    print(f"✓ Created {filename} ({width}x{height})")

if __name__ == '__main__':
    create_png(1024, 1024, 'app-icon.png')
    print("\n✅ Icon generated successfully!")
    print("📝 Run: npx @tauri-apps/cli icon")
