# ZedX 工作台设计 · Preview 05

## 方向

IDEA 的空间布局与工具窗口习惯，Zed 的简洁与编辑区优先。默认深色，提供浅色以评审对比。颜色集中在 CSS 变量，后续便于映射到 GPUI 的主题 token。

## 空间分配

- 顶栏 51px：macOS 窗口控制、主菜单、项目头像与项目名、分支、运行入口、全局搜索、主题。左上角不展示 ZedX 应用品牌。
- 左工具栏 44px：项目、搜索、Git、结构；底部放运行、终端、设置。
- 项目面板默认 265px，支持 180–420px 拖动调节。
- 中间：标签、面包屑、源码预览；保持内容宽度优先。
- 右工具栏 42px：助手、大纲、数据库、通知。右面板按需展开。
- 底部工具窗口默认 202px：终端、运行、问题，可调整高度。
- 状态栏 27px：分支、文件状态、位置、编码与语言。

## 交互规则

1. 点击活动工具窗口按钮再次收起；再次点击恢复。
2. 侧栏状态和内容独立于当前编辑器标签。
3. 多目标导航先用原型中的结构 / 搜索演示定位行为，精确语义留给 LSP。
4. 搜索浮层支持 Escape、上下键和 Enter；保持焦点在浮层内，关闭后归还原焦点。
5. 面板边界支持拖动；图标同时提供标签和工具提示。
6. 桌面是主场景，窄屏将项目树改为可收起覆盖层。

## 原型边界

此文件是单文件 HTML/CSS/JavaScript 原型。源码示例是设计用伪实现，不应直接作为可编译的 GPUI 代码使用。文件树只展示内嵌示例文件；终端不会调用系统命令；Git 提交、AI 和数据库没有真实连接。

## 下一轮评审重点

- 左栏图标尺寸、排序与选中反馈。
- 是否需要工具窗口名称，或仅图标加提示。
- 项目树宽度、代码字号和整体信息密度。
- 是否需要同侧上下两个面板。
- 右侧助手默认关闭是否符合日常使用习惯。

## Preview 02 视觉修订

参考 IDEA 社区版 New UI 和 OpenIDEA 的 `TitleBar`、`ToolWindows` 与主题 token。

- 项目头像为根据项目名生成的 `ZE` 占位图标，与项目名组成一个选择器；不使用 ZedX 品牌图标。后续原生实现可读取项目自定义图标。
- 使用 JetBrains expui 工具窗口与通用图标，以及 OpenIDEA 的 Rust / Cargo 文件图标，内嵌到 HTML；普通工具栏、文件树和标签不再使用手绘替代图标或文字缩写。
- 工作台采用 6px 岛间留白、10px 面板圆角；外部窗口采用 13px 圆角与细边框。
- 顶栏、侧栏外框和工具窗口采用克制的半透明背景及 backdrop-filter，代码区保持实色以保证阅读。浏览器中以背景色模拟桌面透色，不代表原生系统材质已实现。
- 图标来源及许可证见 `THIRD_PARTY_NOTICES.md` 与 `docs/licenses/Apache-2.0.txt`。

参考源码：
- https://github.com/l0ng-ai/openidea/blob/main/src/workbench/TitleBar.tsx
- https://github.com/l0ng-ai/openidea/blob/main/src/workbench/toolwindows/ToolWindows.module.css
- https://github.com/l0ng-ai/openidea/blob/main/src/ui/icons.tsx
- https://www.jetbrains.com/help/idea/new-ui.html

## Preview 03 细节与全局搜索

- 保留 Preview 02 的空间结构，降低窗口外框的透色强度，代码字号提高到 13px / 25px；工具栏、文件树和状态栏文字提高可读性。
- 非活动标签的关闭按钮仅悬停 / 键盘聚焦时出现；选中图标与悬停状态统一。
- 分隔线支持鼠标拖动、键盘箭头微调和双击恢复默认；主题与面板尺寸保存到本地浏览器存储。
- 两次单独轻按 Shift（单次小于 350ms，间隔小于 400ms）打开 Search Everywhere；Shift+字母、修饰键组合及键盘长按不计入手势；失焦清空识别状态。
- 全局搜索包含 All / Files / Symbols / Actions。符号来自内嵌示例文件中声明的名称和行号；不是对真实项目的语义索引。
- 支持名称 / 路径匹配、简单子序列模糊匹配、中文操作别名、关键字高亮、结果路径预览和空结果反馈。
- 上下键选择、Enter 执行、Tab / Shift+Tab 切换分类、Ctrl+1–4 直达分类、Esc 关闭并恢复焦点。Cmd/Ctrl+P 直达 Files。
- 搜索输入采用 combobox / listbox / option 语义，选中项通过 aria-activedescendant 对外暴露；IME 合成时不响应 Enter 导航。

## Preview 04：按 IDEA / OpenIDEA 重写全局搜索（当前设计）

依据 IDEA 2026.2 官方 Search Everywhere 截图与 OpenIDEA `SearchEverywhere.tsx` / CSS 对照实现；此节取代 Preview 03 对全局搜索外观的定义。

- 不设置独立弹窗标题栏、关闭按钮或第二层项目标题；第一行即 All / Classes / Files / Symbols / Actions / Text 分类。
- 右侧为 Include non-project items（Actions 分类为 Include disabled actions）、范围选择器、源码预览、文件类型过滤和打开查找工具窗口。
- 搜索行约 35px，列表行 26px；图标、名称、路径并排，右侧展示模块 / 快捷键。All 搜索按 Classes / Files / Symbols / Actions / Text 分组，空查询展示最近示例文件。
- 默认宽 800px、距离顶部 9vh、圆角 10px，弹窗保留克制半透明材质；遮罩保持透明，不使背景工作台整体模糊或变暗。
- 源码预览可开关，展示选中示例文件及目标附近行；底部显示目标路径与 Open in Right Split，可实际打开右侧只读代码拆分预览。
- 双 Shift 打开，再次双 Shift 或 Alt+N 切换包含非项目项的范围；Tab 循环六分类、上下键选结果、Enter 打开、Shift+Enter 右侧打开、Esc 关闭。
- 支持真实内嵌示例文本匹配、类声明及符号导航、Rust / TOML / Markdown 文件类型过滤，All 中少量结果时补充 Text 命中。
- 原型仍只含项目内嵌示例数据；范围开关表达 UI 状态，不代表已连接外部依赖索引或真实项目搜索。

主要参考：
- https://www.jetbrains.com/help/idea/searching-everywhere.html
- https://resources.jetbrains.com/help/img/idea/2026.2/search_all_window.png
- https://github.com/l0ng-ai/openidea/blob/main/src/features/search/everywhere/SearchEverywhere.tsx
- https://github.com/l0ng-ai/openidea/blob/main/src/features/search/everywhere/SearchEverywhere.module.css

## Preview 05：视觉收尾（当前）

延续 Preview 04 的 IDEA 式结构，仅调整以下细节：

- 36 个有来源的图标使用对应浅色 SVG，并根据主题切换；新增的文件、标签和结果图标也使用当前主题版本。静态工具栏在主题恢复后同步更新。
- 搜索结果名称和路径保持单行，名称占比上限为 68%，路径保留可用空间并使用省略号；完整名称、路径和快捷键可通过原生提示查看。
- 精确匹配与子序列模糊匹配使用统一高亮样式，选中结果的路径对比度适当提高。
- 列表根据实际行数、分组标题与当前视口计算高度；1 条结果保留约 64px 列表区，最近 5 个文件为 162px，不再固定占用 240px。大量结果滚动展示。
- 源码预览、空结果、分类切换和窗口缩放均重新计算高度，保留上下键选择与 Enter 导航行为。
