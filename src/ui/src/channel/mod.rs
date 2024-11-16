pub mod message;
pub mod message_list;

use std::ops::Deref;

use components::input::{InputEvent, TextInput};
use gpui::{div, list, rgb, AppContext, Context, IntoElement, ListAlignment, ListState, Model, ParentElement, Pixels, Render, SharedString, Styled, View, VisualContext};
use message::message;
use message_list::MessageList;
use scope_backend_discord::message::DiscordMessage;
use scope_chat::{channel::Channel, message::Message};

pub struct ChannelView<M: Message + 'static> {
  list_state: ListState,
  list_model: Model<MessageList<M>>,
  message_input: View<TextInput>,
}

impl<M: Message + 'static> ChannelView<M> {
  pub fn create(ctx: &mut gpui::WindowContext, mut channel: impl Channel<Message = M> + 'static) -> View<Self> {
    let view = ctx.new_view(|ctx| {
      let state_model = ctx.new_model(|_cx| MessageList::<M>::new());

      let async_model = state_model.clone();
      let mut async_ctx = ctx.to_async();
      let channel_listener = channel.clone();

      ctx
        .foreground_executor()
        .spawn(async move {
          loop {
            let message = channel.get_receiver().recv().await.unwrap();

            println!("Got Message!");

            async_model
              .update(&mut async_ctx, |data, ctx| {
                data.messages.push(message);
                ctx.notify();
              })
              .unwrap();
          }
        })
        .detach();

      ctx
        .observe(&state_model, |this: &mut ChannelView<M>, model, cx| {
          let items = model.read(cx).messages.clone();

          this.list_state = ListState::new(items.len(), ListAlignment::Bottom, Pixels(20.), move |idx, _cx| {
            let item = items.get(idx).unwrap().clone();
            div().child(message(item)).into_any_element()
          });

          cx.notify();
        })
        .detach();

      let items = state_model.read(ctx).messages.clone();

      let message_input = ctx.new_view(|cx| {
        let mut input = components::input::TextInput::new(cx);
  
        input.set_size(components::Size::Large, cx);
  
        input
      });

      ctx.subscribe(&message_input, move |channel_view, text_input, input_event, ctx| {
        match input_event {
          InputEvent::PressEnter => {
            let content = text_input.read(ctx).text().to_string();
            let channel_sender = channel.clone();

            text_input.update(ctx, |text_input, cx| {
              text_input.set_text("", cx);
            });

            ctx.foreground_executor().spawn(async move {
              let nonce = random_string::generate(20, random_string::charsets::ALPHANUMERIC);

              channel_sender.send_message(content, nonce).await;
            }).detach();
          }
          _ => {}
        }
      }).detach();  
  
      ChannelView::<M> {
        list_state: ListState::new(items.len(), ListAlignment::Bottom, Pixels(20.), move |idx, _cx| {
          let item = items.get(idx).unwrap().clone();
          div().child(message(item)).into_any_element()
        }),
        list_model: state_model,
        message_input,
      }
    });

    view
  }
}

impl<M: Message + 'static> Render for ChannelView<M> {
  fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
    div()
      .flex()
      .flex_col()
      .w_full()
      .h_full()
      .child(list(self.list_state.clone()).w_full().h_full())
      .child(self.message_input.clone())
  }
}
