// Draws the background of the macOS .dmg window: an arrow from Logix IDE to
// Applications, a line telling the user to drag, and one saying how to start
// it afterwards (dragging only copies it). Writes a TIFF with a 1x
// and a 2x (Retina) image.
//
//   swift src-tauri/icons/make-dmg-background.swift src-tauri/icons/dmg-background.tiff
//
// The size and icon positions must match bundle.macOS.dmg in tauri.conf.json
// (windowSize 600x360, Logix IDE at x=160, Applications at x=440, both at y=170;
// Finder positions are icon centers measured from the top left).
import AppKit

let width: CGFloat = 600
let height: CGFloat = 360
let iconY: CGFloat = 170
let teal = NSColor(srgbRed: 0, green: 135 / 255, blue: 143 / 255, alpha: 1)

func draw() {
    // Light background: Finder draws the icon names in dark text over it.
    NSColor(srgbRed: 0.97, green: 0.98, blue: 0.98, alpha: 1).setFill()
    NSRect(x: 0, y: 0, width: width, height: height).fill()

    // Arrow between the two icons (128 px wide, centered at x=160 and 440).
    let y = height - iconY
    let shaft = NSBezierPath()
    shaft.move(to: NSPoint(x: 248, y: y))
    shaft.line(to: NSPoint(x: 340, y: y))
    shaft.lineWidth = 8
    shaft.lineCapStyle = .round
    teal.setStroke()
    shaft.stroke()
    let head = NSBezierPath()
    head.move(to: NSPoint(x: 356, y: y))
    head.line(to: NSPoint(x: 334, y: y + 18))
    head.line(to: NSPoint(x: 334, y: y - 18))
    head.close()
    teal.setFill()
    head.fill()

    // Instructions: the title centered near the top; the two lines at the
    // bottom start at the same x, the block centered on its longer first line.
    func attrs(_ size: CGFloat, _ weight: NSFont.Weight, _ color: NSColor, _ alignment: NSTextAlignment)
        -> [NSAttributedString.Key: Any]
    {
        let style = NSMutableParagraphStyle()
        style.alignment = alignment
        return [
            .font: NSFont.systemFont(ofSize: size, weight: weight),
            .foregroundColor: color,
            .paragraphStyle: style,
        ]
    }
    func text(_ s: String, _ a: [NSAttributedString.Key: Any], size: CGFloat, top: CGFloat, x: CGFloat = 0) {
        let h = size * 1.4
        NSString(string: s).draw(
            in: NSRect(x: x, y: height - top - h, width: width - x, height: h), withAttributes: a)
    }
    text("Install Logix IDE", attrs(22, .semibold, NSColor(white: 0.15, alpha: 1), .center), size: 22, top: 32)
    let line1 = "Drag the Logix IDE icon onto the Applications folder."
    // Dragging only copies it, so say how to start it.
    let line2 = "Then open Logix IDE from Applications in Finder."
    let body = attrs(15, .regular, NSColor(white: 0.3, alpha: 1), .left)
    let left = ((width - NSString(string: line1).size(withAttributes: body).width) / 2).rounded()
    text(line1, body, size: 15, top: 286, x: left)
    text(line2, body, size: 15, top: 310, x: left)
}

func rep(scale: CGFloat) -> NSBitmapImageRep {
    let r = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int(width * scale), pixelsHigh: Int(height * scale),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    r.size = NSSize(width: width, height: height)  // points, so 2x is Retina
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: r)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return r
}

guard CommandLine.arguments.count == 2 else {
    print("usage: swift make-dmg-background.swift <out.tiff>")
    exit(2)
}
let image = NSImage(size: NSSize(width: width, height: height))
image.addRepresentation(rep(scale: 1))
image.addRepresentation(rep(scale: 2))
try! image.tiffRepresentation(using: .lzw, factor: 0)!
    .write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
