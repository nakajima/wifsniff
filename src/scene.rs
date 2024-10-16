use core::{
    cell::{RefCell, RefMut},
    ops::DerefMut,
    result,
};

use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Subscriber},
};
use embassy_time::Timer;
use esp_println::println;

use crate::{button::BUTTON_CHANNEL, lights};

static SCENE_CHANNEL: PubSubChannel<CriticalSectionRawMutex, CurrentScene, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, CurrentScene, 4, 4, 4>::new();

type SceneSubscriber = Subscriber<'static, CriticalSectionRawMutex, CurrentScene, 4, 4, 4>;
type ButtonSubscriber = Subscriber<'static, CriticalSectionRawMutex, ButtonPress, 4, 4, 4>;

pub async fn enter(scene: CurrentScene) {
    SCENE_CHANNEL.publisher().unwrap().publish(scene).await;
}

trait Scene {
    async fn single_press(&mut self) {}
    async fn long_press(&mut self) {}
    async fn enter(&self) {}
    async fn tick(&mut self);
    async fn leave(&self) {}
}

#[derive(Clone, Debug)]
pub enum CurrentScene {
    Startup(StartupScene),
    Sniffing(SniffingScene),
    Menu(MenuScene),
}

impl CurrentScene {
    async fn tick(&mut self) {
        match self {
            Self::Startup(scene) => {
                scene.tick().await;
            }
            Self::Sniffing(scene) => {
                scene.tick().await;
            }
            Self::Menu(scene) => {
                scene.tick().await;
            }
        }
    }

    async fn enter(&self) {
        match self {
            Self::Startup(scene) => scene.enter().await,
            Self::Sniffing(scene) => scene.enter().await,
            Self::Menu(scene) => scene.enter().await,
        }
    }

    async fn leave(&self) {
        match self {
            Self::Startup(scene) => scene.leave().await,
            Self::Sniffing(scene) => scene.leave().await,
            Self::Menu(scene) => scene.leave().await,
        }
    }
}

async fn update_current_scene(subscriber: &mut SceneSubscriber) -> CurrentScene {
    subscriber.next_message_pure().await
}

#[embassy_executor::task]
pub async fn setup_scene_manager() {
    let current_scene: RefCell<CurrentScene> = RefCell::new(CurrentScene::Startup(StartupScene {}));
    current_scene.borrow_mut().enter().await;
    let mut subscriber = SCENE_CHANNEL.subscriber().unwrap();
    let mut button = BUTTON_CHANNEL.subscriber().unwrap();

    async fn listen_to_button(subscribe: &mut ButtonSubscriber) {
        let button = subscribe.next_message_pure().await;
    }

    loop {
        let result = select(
            current_scene.borrow_mut().tick(),
            update_current_scene(&mut subscriber),
        )
        .await;

        match result {
            Either::First(_) => (),
            Either::Second(next_scene) => {
                println!("Scene change: {:?}", next_scene);
                current_scene.borrow().leave().await;
                *current_scene.borrow_mut() = next_scene
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct StartupScene {}

impl Scene for StartupScene {
    async fn enter(&self) {
        lights::off().await;
    }

    async fn tick(&mut self) {
        Timer::after_millis(100).await;
        lights::change(lights::LightChange::White(true)).await;
        Timer::after_millis(200).await;
        lights::change(lights::LightChange::Yellow(true)).await;
        Timer::after_millis(200).await;
        lights::change(lights::LightChange::Green(true)).await;
        Timer::after_millis(200).await;
        lights::change(lights::LightChange::Blue(true)).await;
        Timer::after_millis(400).await;
        lights::off().await;

        enter(CurrentScene::Sniffing(SniffingScene {})).await;
    }
}

#[derive(Clone, Debug)]
pub struct SniffingScene {}

impl Scene for SniffingScene {
    async fn long_press(&mut self) {
        enter(CurrentScene::Menu(MenuScene {
            currentOption: MenuOption::Sniff,
            is_on: true,
        }))
        .await;
    }

    async fn tick(&mut self) {
        Timer::after_millis(2).await;
    }
}

#[derive(Clone, Debug)]
enum MenuOption {
    Bluetooth,
    Sleep,
    Erase,
    Sniff,
}

#[derive(Clone, Debug)]
pub struct MenuScene {
    pub currentOption: MenuOption,
    is_on: bool,
}

impl Scene for MenuScene {
    async fn enter(&self) {
        lights::off().await;
    }

    async fn single_press(&mut self) {
        match self.currentOption {
            MenuOption::Sniff => {
                self.currentOption = MenuOption::Sleep;
            }
            MenuOption::Sleep => {
                self.currentOption = MenuOption::Erase;
            }
            MenuOption::Erase => {
                self.currentOption = MenuOption::Bluetooth;
            }
            MenuOption::Bluetooth => self.currentOption = MenuOption::Sniff,
        }
    }

    async fn tick(&mut self) {
        match self.currentOption {
            MenuOption::Sniff => lights::change(lights::LightChange::White(self.is_on)).await,
            MenuOption::Sleep => lights::change(lights::LightChange::Green(self.is_on)).await,
            MenuOption::Erase => lights::change(lights::LightChange::Yellow(self.is_on)).await,
            MenuOption::Bluetooth => lights::change(lights::LightChange::Blue(self.is_on)).await,
        }

        self.is_on = !self.is_on;

        Timer::after_millis(400).await;
    }
}
