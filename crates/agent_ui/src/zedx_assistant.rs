use super::*;

pub(super) struct ZedxAssistant {
    workspace: WeakEntity<Workspace>,
    editor: Entity<Editor>,
    _workspace_observation: Option<Subscription>,
}

impl AgentPanel {
    pub(super) fn show_zedx_assistant(&self, cx: &App) -> bool {
        if std::env::var_os("ZEDX_UPSTREAM_UI").is_some()
            || !matches!(self.selected_agent, Agent::NativeAgent)
            || LanguageModelRegistry::read_global(cx).has_authenticated_provider(cx)
        {
            return false;
        }
        match &self.base_view {
            BaseView::Uninitialized => true,
            BaseView::AgentThread { conversation_view } => {
                let conversation = conversation_view.read(cx);
                !conversation.has_saved_input()
                    && conversation
                        .active_thread()
                        .is_none_or(|view| view.read(cx).message_editor.read(cx).is_empty(cx))
                    && conversation.as_native_thread(cx).is_some_and(|thread| {
                        let thread = thread.read(cx);
                        thread.is_empty() && thread.draft_prompt().is_none()
                    })
            }
            BaseView::Terminal { .. } => false,
        }
    }
}

impl ZedxAssistant {
    pub(super) fn new(
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            let mut editor = Editor::auto_height(3, 3, window, cx);
            editor.set_placeholder_text("描述你想完成的改动…", window, cx);
            editor
        });
        let observation = workspace
            .upgrade()
            .map(|workspace| cx.observe(&workspace, |_, _, cx| cx.notify()));
        Self {
            workspace,
            editor,
            _workspace_observation: observation,
        }
    }

    fn context(&self, cx: &App) -> (String, String) {
        self.workspace
            .upgrade()
            .and_then(|workspace| {
                let workspace = workspace.read(cx);
                let path = workspace.active_item(cx)?.project_path(cx)?;
                let name = path.path.file_name()?.to_string();
                let parent = path
                    .path
                    .parent()
                    .map(|parent| parent.to_string())
                    .unwrap_or_default();
                Some((name, parent))
            })
            .unwrap_or_else(|| ("未选择文件".into(), "打开文件后自动加入上下文".into()))
    }

    fn send(&self, cx: &mut Context<Self>) {
        struct AssistantNotConfigured;
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.show_toast(
                    workspace::Toast::new(
                        workspace::notifications::NotificationId::unique::<AssistantNotConfigured>(
                        ),
                        "尚未连接 AI 服务，请先在设置中配置模型。",
                    )
                    .autohide(),
                    cx,
                );
            })
            .log_err();
    }
}

impl Focusable for ZedxAssistant {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl Render for ZedxAssistant {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (file, directory) = self.context(cx);
        v_flex()
            .debug_selector(|| "zedx-assistant".into())
            .size_full()
            .min_h_0()
            .bg(cx.theme().colors().panel_background)
            .child(
                v_flex()
                    .id("zedx-assistant-welcome")
                    .overflow_y_scroll()
                    .flex_1()
                    .min_h_0()
                    .p(px(15.0))
                    .gap(px(15.0))
                    .child(
                        Label::new("ZEDX ASSISTANT")
                            .size(ui::zedx::caption_size())
                            .color(Color::Muted),
                    )
                    .child(Label::new("在代码旁边，保持专注。").weight(gpui::FontWeight::SEMIBOLD))
                    .child(
                        div().child(
                            Label::new("讨论当前文件、理解项目结构，或一起完成下一次改动。")
                                .size(ui::zedx::label_size())
                                .color(Color::Muted),
                        ),
                    )
                    .child(
                        v_flex()
                            .debug_selector(|| "zedx-assistant-context".into())
                            .p(px(12.0))
                            .gap_1()
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .rounded(px(7.0))
                            .child(
                                Label::new("当前上下文")
                                    .size(ui::zedx::label_size())
                                    .color(Color::Muted),
                            )
                            .child(
                                Label::new(file)
                                    .size(ui::zedx::label_size())
                                    .color(Color::Accent)
                                    .truncate(),
                            )
                            .child(
                                Label::new(directory)
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .debug_selector(|| "zedx-assistant-compose".into())
                    .m(px(14.0))
                    .p(px(10.0))
                    .gap_2()
                    .flex_none()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .rounded(px(7.0))
                    .child(self.editor.clone())
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                Label::new("Agent · Auto")
                                    .size(ui::zedx::caption_size())
                                    .color(Color::Muted),
                            )
                            .child(
                                div().debug_selector(|| "zedx-assistant-send".into()).child(
                                    ui::ButtonLike::new("zedx-assistant-send")
                                        .width(px(25.0))
                                        .height(px(25.0).into())
                                        .aria_label("发送给助手")
                                        .style(ui::ButtonStyle::Tinted(::ui::TintColor::Accent))
                                        .child(Icon::new(IconName::ArrowUp).size(IconSize::Small))
                                        .tooltip(Tooltip::text("发送"))
                                        .on_click(cx.listener(|this, _, _, cx| this.send(cx))),
                                ),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, size};

    #[gpui::test]
    async fn zedx_assistant_keeps_context_draft_and_composer_visible(cx: &mut TestAppContext) {
        let (workspace, panel, mut cx) = super::super::tests::setup_workspace_panel(cx).await;
        cx.update(|_, cx| {
            LanguageModelRegistry::global(cx).update(cx, |registry, cx| {
                for provider in registry.providers() {
                    registry.unregister_provider(provider.id(), cx);
                }
            })
        });
        workspace.update_in(&mut cx, |workspace, window, cx| {
            workspace.open_panel::<AgentPanel>(window, cx);
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        assert!(cx.debug_bounds("zedx-assistant").is_some());
        let assistant = panel.read_with(&cx, |panel, cx| {
            assert!(panel.show_zedx_assistant(cx));
            panel.zedx_assistant.clone().unwrap()
        });
        assistant.read_with(&cx, |assistant, cx| {
            assert_eq!(assistant.context(cx).0, "未选择文件");
        });
        workspace.update_in(&mut cx, |workspace, window, cx| {
            workspace.focus_panel::<AgentPanel>(window, cx);
            assert!(
                assistant
                    .read(cx)
                    .editor
                    .focus_handle(cx)
                    .is_focused(window)
            );
            assert!(panel.read(cx).focus_handle(cx).contains_focused(window, cx));
        });
        workspace
            .update_in(&mut cx, |workspace, window, cx| {
                workspace.open_abs_path(
                    PathBuf::from("/project/file.txt"),
                    workspace::OpenOptions::default(),
                    window,
                    cx,
                )
            })
            .await
            .unwrap();
        cx.run_until_parked();
        assistant.update_in(&mut cx, |assistant, window, cx| {
            assert_eq!(assistant.context(cx).0, "file.txt");
            assistant
                .editor
                .update(cx, |editor, cx| editor.set_text("解释当前文件", window, cx));
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        let send = cx.debug_bounds("zedx-assistant-send").unwrap();
        cx.simulate_click(send.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert!(
            !workspace
                .read_with(&cx, |workspace, _| workspace.notification_ids())
                .is_empty()
        );
        workspace.update_in(&mut cx, |workspace, window, cx| {
            workspace.close_panel::<AgentPanel>(window, cx);
            workspace.open_panel::<AgentPanel>(window, cx);
        });
        cx.run_until_parked();
        assistant.read_with(&cx, |assistant, cx| {
            assert_eq!(assistant.editor.read(cx).text(cx), "解释当前文件");
        });
        for (width, height) in [(1280.0, 700.0), (390.0, 500.0), (900.0, 300.0)] {
            cx.simulate_resize(size(px(width), px(height)));
            cx.run_until_parked();
            cx.refresh().unwrap();
            let body = cx.debug_bounds("zedx-assistant").unwrap();
            let composer = cx.debug_bounds("zedx-assistant-compose").unwrap();
            assert!(composer.top() >= body.top());
            assert!(composer.bottom() <= body.bottom());
        }
    }

    #[gpui::test]
    async fn zedx_assistant_defers_to_configured_and_external_agents(cx: &mut TestAppContext) {
        let (_, panel, mut cx) = super::super::tests::setup_workspace_panel(cx).await;
        panel.read_with(&cx, |panel, cx| assert!(!panel.show_zedx_assistant(cx)));
        cx.update(|_, cx| {
            LanguageModelRegistry::global(cx).update(cx, |registry, cx| {
                for provider in registry.providers() {
                    registry.unregister_provider(provider.id(), cx);
                }
            })
        });
        panel.update(&mut cx, |panel, cx| {
            assert!(panel.show_zedx_assistant(cx));
            panel.selected_agent = Agent::Custom {
                id: AgentId::from("external"),
            };
            assert!(!panel.show_zedx_assistant(cx));
        });
    }
}
