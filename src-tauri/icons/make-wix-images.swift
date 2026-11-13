// Draws the Windows installer's (.msi, WiX) two images, in place of WiX's
// stock red pictures (the user's pick, 2026-11-13, "A, green panel"):
//
// - the dialog image (493x312), the left part of the welcome and finish
//   windows: a panel in Logix IDE's green (the brand icon's gradient, #10B981
//   to #0F766E) with the icon's white window-and-play glyph and the name. The
//   installer writes its text right of x=164, over white.
// - the banner (493x58), across the top of the steps in between: white, with
//   the brand icon at its right end. The installer writes the step's title
//   over its left part.
//
//   swift src-tauri/icons/make-wix-images.swift src-tauri/icons
//
// Writes wix-dialog.bmp and wix-banner.bmp (24-bit BMP, as WiX needs), which
// bundle.windows.wix's dialogImagePath and bannerPath in tauri.conf.json name.
import AppKit

let green1 = NSColor(srgbRed: 0x10 / 255.0, green: 0xB9 / 255.0, blue: 0x81 / 255.0, alpha: 1)
let green2 = NSColor(srgbRed: 0x0F / 255.0, green: 0x76 / 255.0, blue: 0x6E / 255.0, alpha: 1)

// The brand icon's glyph (docs/brand/ide.svg, viewBox 0 0 64 64), drawn in
// white into the square at (x, y) of side s, in top-left coordinates.
func drawGlyph(x: CGFloat, y: CGFloat, s: CGFloat) {
    let k = s / 64
    NSColor.white.set()
    let window = NSBezierPath(
        roundedRect: NSRect(x: x + 14 * k, y: y + 17 * k, width: 36 * k, height: 30 * k),
        xRadius: 6 * k, yRadius: 6 * k)
    window.lineWidth = 4.5 * k
    window.stroke()
    let play = NSBezierPath()
    play.move(to: NSPoint(x: x + 28 * k, y: y + 25.5 * k))
    play.line(to: NSPoint(x: x + 38.5 * k, y: y + 32 * k))
    play.line(to: NSPoint(x: x + 28 * k, y: y + 38.5 * k))
    play.close()
    play.lineWidth = 3 * k
    play.lineJoinStyle = .round
    play.fill()
    play.stroke()
}

// The diagonal gradient (top left to bottom right) filling `path`: in these
// top-left coordinates, 45 degrees points right and down.
func fillGradient(_ path: NSBezierPath) {
    NSGradient(starting: green1, ending: green2)!.draw(in: path, angle: 45)
}

// The brand icon (tile and glyph) in the square at (x, y) of side s.
func drawIcon(x: CGFloat, y: CGFloat, s: CGFloat) {
    let rect = NSRect(x: x, y: y, width: s, height: s)
    let tile = NSBezierPath(roundedRect: rect, xRadius: 16 * s / 64, yRadius: 16 * s / 64)
    fillGradient(tile)
    drawGlyph(x: x, y: y, s: s)
}

// Draws into a width x height bitmap with top-left coordinates and writes it
// as a 24-bit BMP.
func write(_ path: String, width: Int, height: Int, draw: () -> Void) {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
        bitsPerSample: 8, samplesPerPixel: 3, hasAlpha: false, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 32)!
    let ctx = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx.cgContext, flipped: true)
    ctx.cgContext.translateBy(x: 0, y: CGFloat(height))
    ctx.cgContext.scaleBy(x: 1, y: -1)
    NSColor.white.setFill()
    NSRect(x: 0, y: 0, width: width, height: height).fill()
    draw()
    NSGraphicsContext.restoreGraphicsState()
    // A classic BMP: rows bottom-up, BGR, each padded to 4 bytes (AppKit's
    // own BMP is top-down, which not every reader takes).
    let rowSize = (width * 3 + 3) / 4 * 4
    var out = Data()
    func le32(_ v: Int) { withUnsafeBytes(of: UInt32(v).littleEndian) { out.append(contentsOf: $0) } }
    func le16(_ v: Int) { withUnsafeBytes(of: UInt16(v).littleEndian) { out.append(contentsOf: $0) } }
    out.append(contentsOf: Array("BM".utf8))
    le32(54 + rowSize * height); le32(0); le32(54)
    le32(40); le32(width); le32(height); le16(1); le16(24); le32(0)
    le32(rowSize * height); le32(2835); le32(2835); le32(0); le32(0)
    let pixels = rep.bitmapData!
    for y in (0..<height).reversed() {
        var row = [UInt8](repeating: 0, count: rowSize)
        for x in 0..<width {
            let p = pixels + y * rep.bytesPerRow + x * 4
            row[x * 3] = p[2]
            row[x * 3 + 1] = p[1]
            row[x * 3 + 2] = p[0]
        }
        out.append(contentsOf: row)
    }
    try! out.write(to: URL(fileURLWithPath: path))
}

let dir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."

write("\(dir)/wix-dialog.bmp", width: 493, height: 312) {
    let panel = NSRect(x: 0, y: 0, width: 164, height: 312)
    fillGradient(NSBezierPath(rect: panel))
    drawGlyph(x: 32, y: 70, s: 100)
    let name = "Logix IDE" as NSString
    let attrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 20, weight: .heavy),
        .foregroundColor: NSColor.white,
    ]
    let size = name.size(withAttributes: attrs)
    name.draw(at: NSPoint(x: 82 - size.width / 2, y: 182), withAttributes: attrs)
}

write("\(dir)/wix-banner.bmp", width: 493, height: 58) {
    drawIcon(x: 445, y: 9, s: 40)
}
