# ZedX

基于 Zed 维护的个人 IDE fork。保留编辑器、GPUI 和语言服务基础，优先改造为熟悉的 IDEA 式工作台与工具窗口交互。

## 当前阶段

**P1/P2：原生实现进行中。** 已导入 Zed `v1.26.2`，以独立 `zedx_*` 模块接入工作台、项目头像和 Search Everywhere。HTML 原型继续作为设计参考。

在仓库根目录运行：

```sh
script/zedx check          # 原生编译检查
script/zedx build          # 构建 target/debug/zedx
script/zedx run .          # 打开项目
script/zedx app .          # macOS 本地开发应用
script/zedx upstream-ui .  # 对照上游布局
script/zedx gesture-test   # 双 Shift 防误触测试
script/zedx search-test    # 文件类型和中文别名策略测试
script/zedx integration-test # 原生事件、搜索、面板、诊断、布局和状态栏回归
script/zedx assets-test    # 图标、主题和布局默认值测试
```

原生搜索：双击 Shift 或 Cmd+K；Cmd/Ctrl+P 直达 Files；再次双 Shift 切换范围；Tab / Shift+Tab 切换分类，上下键选择，Enter 打开。文件使用工作树模糊搜索，Classes / Symbols 使用语言服务器，Actions 使用当前窗口可执行命令，Text 使用项目正文搜索。符号结果取决于对应 LSP 是否已启动及其 workspace/symbol 能力。

配置、缓存、数据目录使用 `ZedX` / `zedx` 名称，与官方 Zed 分离；官方自动更新已关闭。当前提供本地二进制与 `target/debug/ZedX.app` 开发应用，尚未制作签名安装包。已接入 39 组明暗图标、岛式圆角和透明模糊主题。搜索分组高亮、多选类型过滤和紧凑项目覆盖模式已接入；已新增 Structure、真实诊断 Problems、任务输出 Run 和状态栏信息；部分工具窗口边界行为及紧凑交互仍需继续对齐，详见 [完整验收清单](PROTOTYPE_PARITY.md)。

合并维护说明：[UPSTREAM.md](UPSTREAM.md)。

- 原型入口：[prototype/index.html](../prototype/index.html)
- 实施计划：[docs/PLAN.md](../docs/PLAN.md)
- 设计说明：[docs/DESIGN.md](../docs/DESIGN.md)
- 当前截图：[docs/preview.png](../docs/preview.png)
- 图标来源：[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)

直接用浏览器打开 `prototype/index.html`，无需安装依赖。也可在项目目录运行：

```sh
python3 -m http.server 4173 --bind 127.0.0.1
```

然后访问 http://127.0.0.1:4173/prototype/ 。

## 原型交互

- 左侧项目 / 搜索 / Git / 结构工具窗口切换、收起。
- 项目文件树展开、文件预览、编辑器标签切换和关闭。
- 底部终端 / 运行 / 问题切换、收起；拖动分隔线调整尺寸。
- 双击 Shift 打开 Search Everywhere，搜索文件 / 符号 / 操作；Cmd/Ctrl+P 直达文件分类。
- 搜索支持上下键、Enter、Tab 分类切换、Escape、模糊匹配与中文操作别名。
- 分隔线可拖动、用方向键微调、双击重置；主题与面板尺寸在本地保存。
- 右侧助手 / 大纲面板，深色 / 浅色主题切换。
- 演示终端支持 `help`、`pwd`、`ls`、`cargo check`、`cargo test`、`clear`。

示例 Rust 代码、诊断、Git 状态和运行输出均用于验证界面，不是真实 ZedX 后端执行结果。AI、数据库、提交功能尚未连接服务。

## 后续方向

上游 remote 已配置为官方仓库；个人远程仓库尚未配置。大面积 UI 替换放入新的 ZedX 模块，原始核心文件保留少量接入口。

## Preview 02

参考 IDEA 社区版 New UI / OpenIDEA：使用项目头像代替应用品牌区，换用来源明确的 expui 图标，增加岛式面板圆角、外框细边与透明模糊材质。

## Preview 03

提高文字对比度、减弱外框透色、统一活动状态。完善双 Shift 手势与全局搜索，支持符号位置导航、分类、键盘选择和焦点恢复。

- 全局搜索截图：[docs/preview-search.png](../docs/preview-search.png)

## Preview 04

全局搜索已按 IDEA / OpenIDEA 重写：All、Classes、Files、Symbols、Actions、Text 六分类，紧凑单行结果，范围与文件类型过滤，可选源码预览以及右侧拆分打开。双 Shift 打开，再次双 Shift 切换范围。

[最近文件](../docs/preview-search-recent.png) · [搜索与源码预览](../docs/preview-search.png)

## Preview 05（当前）

收尾修正了深浅主题图标、搜索结果层级和路径截断，统一精确 / 模糊匹配高亮，搜索弹窗可随结果数量缩短。
