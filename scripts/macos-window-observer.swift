// Read-only observation of the one synthetic App process started by the probe.
// Never requests Screen Recording or Accessibility permission.
import Foundation
import CoreGraphics
import ApplicationServices

guard CommandLine.arguments.count == 2,
      let pid = Int32(CommandLine.arguments[1]), pid > 0 else {
    fputs("Expected one positive App PID\n", stderr)
    exit(2)
}

let raw = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
let matching = (raw as? [[String: Any]] ?? []).filter {
    ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid
}
let windows: [[String: Any]] = matching.map { item in
    var output: [String: Any] = [:]
    for (source, destination) in [
        (kCGWindowNumber as String, "window_id"),
        (kCGWindowLayer as String, "layer"),
        (kCGWindowBounds as String, "bounds"),
        (kCGWindowAlpha as String, "alpha"),
        (kCGWindowIsOnscreen as String, "on_screen"),
        (kCGWindowName as String, "name")
    ] {
        if let value = item[source] { output[destination] = value }
    }
    return output
}

let trusted = AXIsProcessTrusted()
var accessibility: [String: Any] = ["client_trusted": trusted, "permission_requested": false]
if trusted {
    let application = AXUIElementCreateApplication(pid)
    AXUIElementSetMessagingTimeout(application, 1.0)
    var value: CFTypeRef?
    let error = AXUIElementCopyAttributeValue(application, kAXWindowsAttribute as CFString, &value)
    accessibility["windows_error"] = error.rawValue
    let axWindows = value as? [AXUIElement] ?? []
    accessibility["window_count"] = axWindows.count
    var nodes: [[String: Any]] = []
    func visit(_ element: AXUIElement, _ depth: Int) {
        if nodes.count >= 80 || depth > 7 { return }
        var node: [String: Any] = ["depth": depth]
        for (attribute, key) in [(kAXRoleAttribute, "role"), (kAXTitleAttribute, "title"), (kAXDescriptionAttribute, "description")] {
            var field: CFTypeRef?
            let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &field)
            if result == .success, let text = field as? String { node[key] = String(text.prefix(240)) }
        }
        nodes.append(node)
        var children: CFTypeRef?
        if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &children) == .success {
            for child in (children as? [AXUIElement] ?? []).prefix(40) { visit(child, depth + 1) }
        }
    }
    for window in axWindows.prefix(5) { visit(window, 0) }
    accessibility["elements"] = nodes
    accessibility["read_limit"] = ["elements": 80, "depth": 7]
} else {
    accessibility["observation"] = "not_queried_untrusted_client"
}

let report: [String: Any] = [
    "pid": pid,
    "window_server_available": raw != nil,
    "on_screen_windows": windows,
    // Do not infer permission from whether optional window metadata is present.
    "screen_capture_preflight": CGPreflightScreenCaptureAccess(),
    "screen_capture_permission_requested": false,
    "accessibility": accessibility
]
let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys])
FileHandle.standardOutput.write(data)
FileHandle.standardOutput.write(Data("\n".utf8))
