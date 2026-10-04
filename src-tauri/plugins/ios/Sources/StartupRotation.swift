import UIKit
import WebKit

// Full-screen presentation asks UIKit to use the controller's preferred
// orientation. This also works with Tauri's legacy (non-UIScene) windows.
private final class StartupOrientationController: UIViewController {
    let orientation: UIInterfaceOrientation
    var onReady: (() -> Void)?
    var onFailure: ((String) -> Void)?
    private var appeared = false
    private var presentationCompleted = false

    init(orientation: UIInterfaceOrientation) {
        self.orientation = orientation
        super.init(nibName: nil, bundle: nil)
        modalPresentationStyle = .fullScreen
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    override var supportedInterfaceOrientations: UIInterfaceOrientationMask {
        orientation.isLandscape ? .landscapeRight : .portrait
    }

    override var preferredInterfaceOrientationForPresentation: UIInterfaceOrientation { orientation }
    override var shouldAutorotate: Bool { true }

    override func loadView() {
        view = UIView()
        view.backgroundColor = .systemBackground
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        appeared = true
        notifyWhenReady()
    }

    func markPresented() {
        presentationCompleted = true
        notifyWhenReady()
        if onReady != nil, #available(iOS 16.0, *), let scene = view.window?.windowScene {
            setNeedsUpdateOfSupportedInterfaceOrientations()
            scene.requestGeometryUpdate(.iOS(interfaceOrientations: supportedInterfaceOrientations)) { [weak self] error in
                self?.onFailure?(error.localizedDescription)
            }
        }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        notifyWhenReady()
    }

    private func notifyWhenReady() {
        guard appeared, presentationCompleted, view.window != nil,
              view.bounds.width > 0, view.bounds.height > 0 else { return }
        let landscape = view.bounds.width > view.bounds.height
        guard landscape == orientation.isLandscape, let ready = onReady else { return }
        onReady = nil
        // Start the next presentation after UIKit finishes this appearance callback.
        DispatchQueue.main.async(execute: ready)
    }
}

final class StartupRotation {
    private(set) var completedOrientations: [UIInterfaceOrientation] = []
    private weak var webview: WKWebView?
    private weak var presenter: UIViewController?
    private var landscape: StartupOrientationController?
    private var portrait: StartupOrientationController?
    private var timeout: DispatchWorkItem?
    private var inactiveObserver: NSObjectProtocol?
    private var started = false
    private var finished = false

    func start(webview: WKWebView) {
        guard !started else { return }
        started = true
        self.webview = webview
        guard UIDevice.current.userInterfaceIdiom == .phone else { return }
        // The HTML already pads env(safe-area-inset-*). UIKit must not subtract
        // the same safe areas from the WebKit viewport as scroll-view insets.
        webview.scrollView.contentInsetAdjustmentBehavior = .never
        webview.scrollView.contentInset = .zero
        webview.scrollView.scrollIndicatorInsets = .zero
        webview.scrollView.automaticallyAdjustsScrollIndicatorInsets = false
        waitForWindow(attempts: 40)
    }

    private func waitForWindow(attempts: Int) {
        guard !finished else { return }
        guard let window = webview?.window, window.isKeyWindow,
              UIApplication.shared.applicationState == .active,
              let root = window.rootViewController, root.view.window === window,
              root.presentedViewController == nil else {
            guard attempts > 0 else {
                finish(reason: "startup window was not ready")
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
                self?.waitForWindow(attempts: attempts - 1)
            }
            return
        }
        let supported = UIApplication.shared.supportedInterfaceOrientations(for: window)
        guard supported.contains([.landscapeRight, .portrait]) else {
            finish(reason: "app does not support landscape and portrait")
            return
        }
        presenter = root
        inactiveObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.willResignActiveNotification, object: nil, queue: .main
        ) { [weak self] _ in self?.finish(reason: "app became inactive") }
        let controller = StartupOrientationController(orientation: .landscapeRight)
        landscape = controller
        controller.onReady = { [weak self] in self?.returnToPortrait() }
        controller.onFailure = { [weak self] error in self?.finish(reason: error) }
        let deadline = DispatchWorkItem { [weak self] in self?.finish(reason: "rotation timed out") }
        timeout = deadline
        DispatchQueue.main.asyncAfter(deadline: .now() + 4, execute: deadline)
        NSLog("[StartupRotation] requesting landscape then portrait")
        root.present(controller, animated: false) { controller.markPresented() }
    }

    private func returnToPortrait() {
        guard !finished, let landscape = landscape else { return }
        completedOrientations.append(.landscapeRight)
        NSLog("[StartupRotation] landscape layout completed")
        let controller = StartupOrientationController(orientation: .portrait)
        portrait = controller
        controller.onReady = { [weak self] in self?.finish(reason: nil) }
        controller.onFailure = { [weak self] error in self?.finish(reason: error) }
        landscape.present(controller, animated: false) { controller.markPresented() }
    }

    private func finish(reason: String?) {
        guard !finished else { return }
        finished = true
        timeout?.cancel()
        timeout = nil
        if let observer = inactiveObserver { NotificationCenter.default.removeObserver(observer) }
        inactiveObserver = nil
        landscape?.onReady = nil
        portrait?.onReady = nil
        landscape?.onFailure = nil
        portrait?.onFailure = nil
        if let reason = reason { NSLog("[StartupRotation] stopped: %@", reason) }
        else {
            completedOrientations.append(.portrait)
            NSLog("[StartupRotation] portrait layout completed")
        }

        let restored = { [weak self] in
            guard let self = self else { return }
            self.landscape = nil
            self.portrait = nil
            guard let webview = self.webview, let parent = webview.superview else { return }
            // Reattach/layout is complete before notifying the existing JS viewport handler.
            parent.setNeedsLayout()
            parent.layoutIfNeeded()
            webview.frame = parent.bounds
            webview.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            webview.setNeedsLayout()
            webview.layoutIfNeeded()
            webview.evaluateJavaScript("window.dispatchEvent(new Event('resize'))", completionHandler: nil)
        }
        if let root = presenter, root.presentedViewController === landscape, landscape != nil {
            root.dismiss(animated: false, completion: restored)
        } else {
            restored()
        }
    }
}
