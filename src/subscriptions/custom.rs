use crate::entries::Entry;
use async_process::{Command, Stdio};
use iced::futures;
use iced::futures::channel::mpsc;
use iced::futures::{io::BufReader, prelude::*};
use iced_native::futures::stream::BoxStream;
use iced_native::futures::StreamExt;
use iced_native::Subscription;
use std::hash::Hash;

pub struct ExternalCommandSubscription {
    command: String,
}

impl ExternalCommandSubscription {
    pub fn subscription(command: &str) -> Subscription<Vec<Entry>> {
        iced::Subscription::from_recipe(ExternalCommandSubscription {
            command: command.to_string(),
        })
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for ExternalCommandSubscription
where
    H: std::hash::Hasher,
{
    type Output = Vec<Entry>;

    fn hash(&self, state: &mut H) {
        std::any::TypeId::of::<Self>().hash(state);
        self.command.hash(state)
    }

    fn stream(self: Box<Self>, _: BoxStream<I>) -> BoxStream<Self::Output> {
        let (sender, receiver) = mpsc::channel(100000);
        let command = self.command.clone();
        async_std::task::spawn(run_process(sender, command));
        Box::pin(receiver)
    }
}

async fn run_process(mut sender: futures::channel::mpsc::Sender<Vec<Entry>>, args: String) {
    let args = shell_words::split(&args).unwrap();

    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut chunks = lines.chunks(100);

    while let Some(chunk) = chunks.next().await {
        let mut next_batch = Vec::with_capacity(100);
        for entry in chunk {
            next_batch.push(Entry::from_custom_entry(entry.unwrap()))
        }
        sender.start_send(next_batch).unwrap();
    }
}
