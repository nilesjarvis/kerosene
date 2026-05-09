use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, button, checkbox, container, rule, scrollable, stack};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_macro_indicator_menu(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let mut menu_col = Column::new().spacing(4).padding(8);

        macro_rules! add_cb {
            ($label:expr, $key:expr, $is_checked:expr) => {
                let k = $key.to_string();
                let cb = checkbox($is_checked)
                    .label($label)
                    .on_toggle(move |_| Message::ToggleMacroIndicator(chart_id, k.clone()))
                    .size(12);
                menu_col = menu_col.push(cb);
            };
        }

        add_cb!(
            "TF 50 SMA",
            "tf_sma_50",
            instance.macro_indicators.tf_sma_50
        );
        add_cb!(
            "TF 50 EMA",
            "tf_ema_50",
            instance.macro_indicators.tf_ema_50
        );
        add_cb!(
            "TF 200 SMA",
            "tf_sma_200",
            instance.macro_indicators.tf_sma_200
        );
        add_cb!(
            "TF 200 EMA",
            "tf_ema_200",
            instance.macro_indicators.tf_ema_200
        );
        menu_col = menu_col.push(rule::horizontal(1));
        add_cb!("50d SMA", "sma_50d", instance.macro_indicators.sma_50d);
        add_cb!("50d EMA", "ema_50d", instance.macro_indicators.ema_50d);
        add_cb!("200d SMA", "sma_200d", instance.macro_indicators.sma_200d);
        add_cb!("200d EMA", "ema_200d", instance.macro_indicators.ema_200d);
        menu_col = menu_col.push(rule::horizontal(1));
        add_cb!("20w SMA", "sma_20w", instance.macro_indicators.sma_20w);
        add_cb!("20w EMA", "ema_20w", instance.macro_indicators.ema_20w);
        add_cb!("50w SMA", "sma_50w", instance.macro_indicators.sma_50w);
        add_cb!("50w EMA", "ema_50w", instance.macro_indicators.ema_50w);
        menu_col = menu_col.push(rule::horizontal(1));
        add_cb!("12M SMA", "sma_12m", instance.macro_indicators.sma_12m);
        add_cb!("12M EMA", "ema_12m", instance.macro_indicators.ema_12m);
        menu_col = menu_col.push(rule::horizontal(1));
        add_cb!(
            "Show Labels",
            "show_labels",
            instance.macro_indicators.show_labels
        );

        let menu_card = container(scrollable(menu_col).height(iced::Length::Shrink))
            .width(iced::Length::Shrink)
            .max_height(250.0)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let bg_overlay = button("")
            .width(Fill)
            .height(Fill)
            .on_press(Message::CloseAllMenus)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            });

        stack![
            bg_overlay,
            container(menu_card)
                .width(Fill)
                .height(Fill)
                .padding([32, 20])
                .align_x(iced::Alignment::Start)
                .align_y(iced::Alignment::Start)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}
