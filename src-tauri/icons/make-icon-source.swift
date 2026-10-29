// Draws the app icon source (1024x1024 PNG) from the Open Logix brand icon for
// Logix IDE (openlogix.github.io docs/brand/ide.svg): a green gradient tile
// with a white window and play button. The 64x64 brand drawing is scaled to
// Apple's ~824 px icon art, centered on a transparent 1024 px canvas.
//
// Usage: swift src-tauri/icons/make-icon-source.swift src-tauri/icons/icon-source.png
// Then:  npx tauri icon src-tauri/icons/icon-source.png
import AppKit
import ImageIO
import UniformTypeIdentifiers

let size = 1024
let art = 824.0
let margin = (Double(size) - art) / 2
let scale = art / 64.0  // brand units (viewBox 0 0 64 64) to pixels

func rgb(_ hex: UInt32) -> CGColor {
    CGColor(
        srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
        green: CGFloat((hex >> 8) & 0xFF) / 255,
        blue: CGFloat(hex & 0xFF) / 255,
        alpha: 1)
}

let cs = CGColorSpace(name: CGColorSpace.sRGB)!
let ctx = CGContext(
    data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

// Work in brand units with the y axis pointing down, as in the SVG.
ctx.translateBy(x: 0, y: CGFloat(size))
ctx.scaleBy(x: 1, y: -1)
ctx.translateBy(x: margin, y: margin)
ctx.scaleBy(x: scale, y: scale)

// Tile: rect 64x64, rx 16, diagonal gradient #10B981 -> #0F766E.
let tile = CGPath(
    roundedRect: CGRect(x: 0, y: 0, width: 64, height: 64),
    cornerWidth: 16, cornerHeight: 16, transform: nil)
ctx.saveGState()
ctx.addPath(tile)
ctx.clip()
let gradient = CGGradient(
    colorsSpace: cs, colors: [rgb(0x10B981), rgb(0x0F766E)] as CFArray,
    locations: [0, 1])!
ctx.drawLinearGradient(
    gradient, start: CGPoint(x: 0, y: 0), end: CGPoint(x: 64, y: 64), options: [])
ctx.restoreGState()

// Window: rect x14 y17 w36 h30 rx6, white stroke 4.5, round joins.
let white = rgb(0xFFFFFF)
ctx.setStrokeColor(white)
ctx.setFillColor(white)
ctx.setLineCap(.round)
ctx.setLineJoin(.round)
ctx.setLineWidth(4.5)
ctx.addPath(
    CGPath(
        roundedRect: CGRect(x: 14, y: 17, width: 36, height: 30),
        cornerWidth: 6, cornerHeight: 6, transform: nil))
ctx.strokePath()

// Play button: triangle (28,25.5) (38.5,32) (28,38.5), filled and stroked 3.
let play = CGMutablePath()
play.move(to: CGPoint(x: 28, y: 25.5))
play.addLine(to: CGPoint(x: 38.5, y: 32))
play.addLine(to: CGPoint(x: 28, y: 38.5))
play.closeSubpath()
ctx.setLineWidth(3)
ctx.addPath(play)
ctx.drawPath(using: .fillStroke)

let out = URL(fileURLWithPath: CommandLine.arguments[1])
let dest = CGImageDestinationCreateWithURL(
    out as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(dest, ctx.makeImage()!, nil)
guard CGImageDestinationFinalize(dest) else { fatalError("can't write \(out.path)") }
