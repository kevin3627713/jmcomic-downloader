import Foundation
import SwiftRs
import Tauri
import UIKit
import QuickLook
import WebKit

private struct OpenFilesArgs: Decodable {
    let paths: [String]
    let preview: Bool
}

final class FileActionsPlugin: Plugin, QLPreviewControllerDataSource {
    private var previewURLs: [URL] = []
    private weak var hostWebview: WKWebView?
    private let startupRotation = StartupRotation()

    @objc public override func load(webview: WKWebView) {
        hostWebview = webview
        DispatchQueue.main.async { [weak self, weak webview] in
            guard let webview = webview else { return }
            self?.startupRotation.start(webview: webview)
        }
    }

    @objc public func openFiles(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(OpenFilesArgs.self)
        let urls = args.paths.map { URL(fileURLWithPath: $0) }
        guard !urls.isEmpty, urls.allSatisfy({ FileManager.default.isReadableFile(atPath: $0.path) }) else {
            invoke.reject("导出文件不存在或无法读取")
            return
        }
        DispatchQueue.main.async {
            guard var presenter = self.hostWebview?.window?.rootViewController else {
                invoke.reject("无法打开系统文件面板")
                return
            }
            while let presented = presenter.presentedViewController {
                presenter = presented
            }
            guard !(presenter is UIActivityViewController), !(presenter is QLPreviewController), !presenter.isBeingDismissed else {
                invoke.reject("请先关闭当前预览或分享面板")
                return
            }
            if args.preview {
                self.previewURLs = urls
                let preview = QLPreviewController()
                preview.dataSource = self
                presenter.present(preview, animated: true) { invoke.resolve() }
            } else {
                let sheet = UIActivityViewController(activityItems: urls, applicationActivities: nil)
                if let popover = sheet.popoverPresentationController {
                    popover.sourceView = presenter.view
                    popover.sourceRect = CGRect(x: presenter.view.bounds.midX, y: presenter.view.bounds.maxY - 1, width: 1, height: 1)
                    popover.permittedArrowDirections = []
                }
                presenter.present(sheet, animated: true) { invoke.resolve() }
            }
        }
    }

    func numberOfPreviewItems(in controller: QLPreviewController) -> Int { previewURLs.count }

    func previewController(_ controller: QLPreviewController, previewItemAt index: Int) -> QLPreviewItem {
        previewURLs[index] as NSURL
    }
}

@_cdecl("init_plugin_file_actions")
func initPlugin() -> Plugin { FileActionsPlugin() }
