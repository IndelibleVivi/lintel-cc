// Finite native-AX evidence helper. Searches only the launched App's windows.
// No global events, UI coordinates, permission requests or WebKit injection.
import Foundation
import ApplicationServices

func emit(_ value: [String: Any]) {
    let bytes = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(bytes)
    FileHandle.standardOutput.write(Data("\n".utf8))
}
guard CommandLine.arguments.count >= 3, let pid = Int32(CommandLine.arguments[1]), pid > 0 else {
    emit(["error": "Expected App PID and finite command"]); exit(2)
}
let command = CommandLine.arguments[2]
guard ["snapshot", "press", "cancel"].contains(command) else { emit(["error": "Unsupported command"]); exit(2) }
guard AXIsProcessTrusted() else { emit(["pid": pid, "trusted": false, "performed": false]); exit(0) }
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}
var elements: [(AXUIElement, [String: Any])] = []
func visit(_ element: AXUIElement, _ depth: Int) {
    if elements.count >= 400 || depth > 14 { return }
    var row: [String: Any] = ["depth": depth]
    for (field, key) in [(kAXRoleAttribute, "role"), (kAXTitleAttribute, "title"), (kAXDescriptionAttribute, "description"), (kAXValueAttribute, "value")] {
        let value = attribute(element, field)
        if let text = value as? String { row[key] = String(text.prefix(320)) }
        else if let number = value as? NSNumber { row[key] = number }
    }
    if let enabled = attribute(element, kAXEnabledAttribute) as? Bool { row["enabled"] = enabled }
    var actions: CFArray?
    if AXUIElementCopyActionNames(element, &actions) == .success { row["actions"] = actions as? [String] ?? [] }
    elements.append((element, row))
    for child in (attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []).prefix(80) { visit(child, depth + 1) }
}
let windows = attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
for window in windows.prefix(5) { visit(window, 0) }
if command == "snapshot" {
    emit(["pid": pid, "trusted": true, "elements": elements.map { $0.1 }, "limit": 400]); exit(0)
}
guard CommandLine.arguments.count == 5 else { emit(["error": "Expected exact role and title"]); exit(2) }
let role = CommandLine.arguments[3], title = CommandLine.arguments[4]
let allowed: [String: Set<String>] = [
    "AXCheckBox": ["Light Day", "Dark Night", "Follow system"],
    "AXButton": ["Records & recovery", "Preserve work", "Environments", "Start a task", "Close panel"],
    "AXPopUpButton": ["Interface language", "界面语言"],
    "AXMenuItem": ["简体中文", "English", "Follow system", "跟随系统"]
]
guard allowed[role]?.contains(title) == true, command != "cancel" || role == "AXPopUpButton" else {
    emit(["error": "Action is outside the finite probe scope"]); exit(2)
}
let matches = elements.filter { $0.1["role"] as? String == role && $0.1["title"] as? String == title }
let action = command == "press" ? kAXPressAction : kAXCancelAction
guard matches.count == 1, matches[0].1["enabled"] as? Bool != false,
      (matches[0].1["actions"] as? [String] ?? []).contains(action) else {
    emit(["pid": pid, "trusted": true, "performed": false, "matches": matches.count, "role": role, "title": title, "reason": "exact enabled supported action unavailable"]); exit(0)
}
let error = AXUIElementPerformAction(matches[0].0, action as CFString)
emit(["pid": pid, "trusted": true, "performed": error == .success, "error_code": error.rawValue, "role": role, "title": title, "action": action])
