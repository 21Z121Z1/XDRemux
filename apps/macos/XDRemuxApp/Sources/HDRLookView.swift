import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct HDRLookView: View {
    @Bindable var viewModel: HDRLookViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Button("添加照片或文件夹", systemImage: "plus", action: chooseInputs)
                Button("输出目录", systemImage: "folder", action: chooseOutput)
                Picker("HDR 亮度余量", selection: $viewModel.headroom) {
                    ForEach([2, 4, 8], id: \.self) { Text("\($0)×").tag($0) }
                }
                .frame(width: 180)
                Spacer()
                Button("清空", systemImage: "trash", action: viewModel.clear)
            }
            .disabled(viewModel.isRunning)

            Text(viewModel.outputDirectory?.path ?? "在首个输入旁创建新的 HDR-Look 文件夹")
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)

            if let receipt = viewModel.receipt {
                Table(receipt.results) {
                    TableColumn("原图") { Text(URL(fileURLWithPath: $0.input).lastPathComponent) }
                    TableColumn("状态") { Text($0.status == "success" ? "完成" : "失败") }
                        .width(60)
                    TableColumn("结果") { item in
                        if let error = item.error {
                            Text(error).foregroundStyle(.red).help(error)
                        } else {
                            Button(URL(fileURLWithPath: item.output).lastPathComponent) {
                                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: item.output)])
                            }
                            .buttonStyle(.link)
                        }
                    }
                }
            } else if viewModel.inputURLs.isEmpty {
                ContentUnavailableView("拖入原始摄影风格照片", systemImage: "photo.badge.plus",
                    description: Text("支持单张照片和文件夹批量处理。输出保留原图尺寸，仅保存 HDR HEIC。"))
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(viewModel.inputURLs, id: \.path) { Text($0.path).textSelection(.enabled) }
            }

            if let error = viewModel.errorMessage {
                Text(error).foregroundStyle(.red).textSelection(.enabled)
            }
            HStack {
                if viewModel.isRunning {
                    ProgressView().controlSize(.small)
                    Text("正在重建 HDR；大尺寸照片需要一些时间…")
                } else if let receipt = viewModel.receipt {
                    Text("完成 \(receipt.results.count - receipt.failures) 张，失败 \(receipt.failures) 张")
                    Button("在 Finder 中打开") {
                        NSWorkspace.shared.open(URL(fileURLWithPath: receipt.outputDirectory))
                    }
                } else {
                    Text("\(viewModel.inputURLs.count) 个输入 · 原图保留")
                }
                Spacer()
                Button("生成 HDR 照片", systemImage: "sparkles", action: viewModel.run)
                    .buttonStyle(.borderedProminent)
                    .disabled(!viewModel.canRun)
            }
        }
        .padding(20)
        .dropDestination(for: URL.self) { urls, _ in
            guard !viewModel.isRunning, !urls.isEmpty else { return false }
            viewModel.addInputs(urls)
            return true
        }
    }

    private func chooseInputs() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowedContentTypes = [.heic, .heif]
        if panel.runModal() == .OK { viewModel.addInputs(panel.urls) }
    }

    private func chooseOutput() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        if panel.runModal() == .OK { viewModel.outputDirectory = panel.url }
    }
}
