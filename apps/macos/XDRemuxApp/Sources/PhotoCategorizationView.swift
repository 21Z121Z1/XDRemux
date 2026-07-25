import AppKit
import SwiftUI
import UniformTypeIdentifiers
import XDRemuxCore

struct PhotoCategorizationView: View {
    let viewModel: PhotoCategorizationViewModel

    var body: some View {
        VStack(spacing: 0) {
            controls
            Divider()
            if viewModel.items.isEmpty {
                ContentUnavailableView(
                    "选择照片并扫描",
                    systemImage: "square.grid.2x2",
                    description: Text(statusText)
                )
            } else {
                Table(viewModel.items) {
                    TableColumn("文件") { item in
                        Text(item.sourceURL.lastPathComponent)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    TableColumn("拍摄模式") { item in
                        Text(item.classification.mode?.folderName ?? "根目录")
                    }
                    TableColumn("状态") { item in
                        Text(item.classification.mode == nil
                            ? item.classification.status.appDisplayName
                            : item.disposition.displayName)
                    }
                    TableColumn("目标") { item in
                        Text(item.destinationURL.path)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
                .tableStyle(.inset)
            }
            Divider()
            footer
        }
        .frame(minWidth: 1040, minHeight: 680)
        .dropDestination(for: URL.self) { urls, _ in
            viewModel.addInputs(urls)
            return !urls.isEmpty
        }
    }

    private var controls: some View {
        HStack(spacing: 10) {
            Button(action: chooseInputs) {
                Label("添加照片", systemImage: "plus")
            }
            .disabled(viewModel.isBusy)

            Button(action: chooseOutputDirectory) {
                Label("目标目录", systemImage: "folder")
            }
            .disabled(viewModel.isBusy)

            Text(viewModel.outputDirectory?.path ?? "各照片所在目录")
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)

            Spacer()

            Button {
                viewModel.scan()
            } label: {
                Label("扫描", systemImage: "magnifyingglass")
            }
            .disabled(!viewModel.canScan)

            Button {
                viewModel.copyPlannedFiles()
            } label: {
                Label("开始分类", systemImage: "play.fill")
            }
            .buttonStyle(.borderedProminent)
            .disabled(!viewModel.canCopy)

            Button {
                viewModel.cancel()
            } label: {
                Image(systemName: "stop.fill")
            }
            .help("取消")
            .disabled(!viewModel.isBusy)

            Button(action: viewModel.revealResults) {
                Image(systemName: "folder.badge.magnifyingglass")
            }
            .help("在 Finder 中显示结果")
            .disabled(viewModel.items.allSatisfy {
                $0.disposition != .copied && $0.disposition != .duplicate
            })

            Button {
                viewModel.clear()
            } label: {
                Image(systemName: "trash")
            }
            .help("清空")
            .disabled(viewModel.isBusy || (viewModel.inputURLs.isEmpty && viewModel.items.isEmpty))
        }
        .padding(14)
    }

    private var footer: some View {
        HStack(spacing: 18) {
            Label("\(viewModel.items.isEmpty ? viewModel.inputURLs.count : viewModel.items.count) 张照片", systemImage: "photo.on.rectangle")
            Label("\(viewModel.categorizedCount) 已分类", systemImage: "folder.badge.gearshape")
                .help(viewModel.modeSummary)
            Label("\(viewModel.rootCount) 根目录", systemImage: "tray")
            Label("\(viewModel.duplicateCount) 重复", systemImage: "equal.circle")
            if viewModel.failedCount > 0 {
                Label("\(viewModel.failedCount) 失败", systemImage: "exclamationmark.triangle")
                    .foregroundStyle(.red)
            }
            Spacer()
            Text(statusText)
                .foregroundStyle(.secondary)
                .help(viewModel.modeSummary)
        }
        .font(.caption)
        .padding(.horizontal, 14)
        .frame(height: 42)
    }

    private var statusText: String {
        switch viewModel.state {
        case .idle: return "等待扫描"
        case .scanning: return "正在读取 UserComment"
        case .ready: return "扫描完成"
        case .copying: return "正在复制 \(viewModel.completedCount)/\(viewModel.items.count)"
        case .completed: return "分类完成"
        case .cancelled: return "已取消"
        case .failed(let message): return message
        }
    }

    private func chooseInputs() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowedContentTypes = [.heic, .heif, .jpeg]
        panel.prompt = "添加"
        if panel.runModal() == .OK { viewModel.addInputs(panel.urls) }
    }

    private func chooseOutputDirectory() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.prompt = "选择"
        if panel.runModal() == .OK { viewModel.outputDirectory = panel.url }
    }
}

private extension PhotoCategorizationDisposition {
    var displayName: String {
        switch self {
        case .copy: return "待复制"
        case .duplicate: return "内容相同，跳过"
        case .copied: return "已复制"
        case .failed: return "失败"
        case .dryRun: return "预演"
        }
    }
}
