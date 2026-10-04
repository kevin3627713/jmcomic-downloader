import UIKit
import WebKit

// A separate simulator app: no Tauri setup, comic API, or user files.
@main
final class RotationTestAppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    private let rotation = StartupRotation()
    private var webview: WKWebView!

    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        let root = UIViewController()
        window.rootViewController = root
        self.window = window
        root.loadViewIfNeeded()
        // Start with a genuinely undersized native WebView, not a mocked JS height.
        var frame = window.bounds
        frame.size.height -= 80
        webview = WKWebView(frame: frame)
        webview.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        root.view.addSubview(webview)
        webview.loadHTMLString("""
            <!doctype html><html><head>
            <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
            <style>body{margin:0}.shell{position:fixed;inset:0}.nav{position:absolute;bottom:0;height:60px}</style>
            </head><body><div class="shell"><div class="nav">Startup layout test</div></div></body></html>
            """, baseURL: nil)
        window.makeKeyAndVisible()
        DispatchQueue.main.async {
            self.rotation.start(webview: self.webview)
            self.rotation.start(webview: self.webview)
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { self.verify() }
        return true
    }

    private func verify() {
        guard let window = window, let root = window.rootViewController else {
            saveResult(["status": "failed", "reason": "missing window"])
            return
        }
        guard root.presentedViewController == nil else {
            saveResult(["status": "failed", "reason": "temporary controller remained visible"])
            return
        }
        guard rotation.completedOrientations == [.landscapeRight, .portrait] else {
            saveResult(["status": "failed", "reason": "native orientation sequence did not complete",
                        "orientations": rotation.completedOrientations.map { $0.rawValue }])
            return
        }
        guard window.bounds.height > window.bounds.width,
              webview.window === window,
              abs(webview.frame.height - root.view.bounds.height) < 1,
              abs(webview.frame.width - root.view.bounds.width) < 1 else {
            saveResult(["status": "failed", "reason": "portrait WebView did not fill the root view"])
            return
        }
        // Once completed, another load/start must not present anything again.
        rotation.start(webview: webview)
        let expected = Double(webview.bounds.height)
        webview.evaluateJavaScript("""
            ({layoutHeight:document.documentElement.clientHeight,
              visualHeight:window.visualViewport.height,
              navBottom:document.querySelector('.nav').getBoundingClientRect().bottom})
            """) { value, error in
                guard error == nil, let metrics = value as? [String: Any],
                      let height = metrics["layoutHeight"] as? NSNumber,
                      let bottom = metrics["navBottom"] as? NSNumber,
                      let visualHeight = metrics["visualHeight"] as? NSNumber,
                      abs(height.doubleValue - expected) < 1,
                      abs(bottom.doubleValue - expected) < 1,
                      abs(visualHeight.doubleValue - expected) < 1 else {
                    self.saveResult(["status": "failed", "reason": "WebKit viewport did not recover", "metrics": value ?? NSNull(), "expectedHeight": expected])
                    return
                }
                self.saveResult(["status": "passed", "modalDismissed": true, "portrait": true,
                                 "webviewFillsRoot": true, "metrics": metrics, "expectedHeight": expected,
                                 "orientations": self.rotation.completedOrientations.map {
                                     $0 == .landscapeRight ? "landscapeRight" : "portrait"
                                 }])
            }
    }

    private func saveResult(_ result: [String: Any]) {
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let data = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
            try data.write(to: directory.appendingPathComponent("startup-rotation-result.json"), options: .atomic)
        } catch { NSLog("[StartupRotationTest] result write failed: %@", error.localizedDescription) }
    }
}
