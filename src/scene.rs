use core::cell::RefCell;

use embassy_futures::select::{select3, Either3};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Subscriber},
};
use embassy_time::Timer;
use esp_println::println;

use crate::{
    button::{ButtonPress, BUTTON_CHANNEL},
    lights,
};

static SCENE_CHANNEL: PubSubChannel<CriticalSectionRawMutex, CurrentScene, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, CurrentScene, 4, 4, 4>::new();

type SceneSubscriber = Subscriber<'static, CriticalSectionRawMutex, CurrentScene, 4, 4, 4>;
type ButtonSubscriber = Subscriber<'static, CriticalSectionRawMutex, ButtonPress, 4, 4, 4>;

pub async fn enter(scene: CurrentScene) {
    SCENE_CHANNEL.publisher().unwrap().publish(scene).await;
}

trait Scene {
    async fn button_press(&mut self) {}
    async fn button_down(&mut self) {}
    async fn button_up(&mut self) {}
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

    async fn button_press(&mut self) {
        match self {
            Self::Startup(scene) => {
                scene.button_press().await;
            }
            Self::Sniffing(scene) => {
                scene.button_press().await;
            }
            Self::Menu(scene) => {
                scene.button_press().await;
            }
        }
    }

    async fn button_down(&mut self) {
        match self {
            Self::Startup(scene) => {
                scene.button_down().await;
            }
            Self::Sniffing(scene) => {
                scene.button_down().await;
            }
            Self::Menu(scene) => {
                scene.button_down().await;
            }
        }
    }

    async fn button_up(&mut self) {
        match self {
            Self::Startup(scene) => {
                scene.button_up().await;
            }
            Self::Sniffing(scene) => {
                scene.button_up().await;
            }
            Self::Menu(scene) => {
                scene.button_up().await;
            }
        }
    }

    async fn long_press(&mut self) {
        match self {
            Self::Startup(scene) => {
                scene.long_press().await;
            }
            Self::Sniffing(scene) => {
                scene.long_press().await;
            }
            Self::Menu(scene) => {
                scene.long_press().await;
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

    loop {
        let result = select3(
            current_scene.borrow_mut().tick(),
            update_current_scene(&mut subscriber),
            button.next_message_pure(),
        )
        .await;

        match result {
            Either3::First(_) => (),
            Either3::Second(next_scene) => {
                println!("Scene change: {:?}", next_scene);
                current_scene.borrow().leave().await;
                *current_scene.borrow_mut() = next_scene
            }
            Either3::Third(button_press) => match button_press {
                ButtonPress::Single => {
                    current_scene.borrow_mut().button_press().await;
                }
                ButtonPress::Long => {
                    current_scene.borrow_mut().long_press().await;
                }
                ButtonPress::Down => {
                    current_scene.borrow_mut().button_down().await;
                }
                ButtonPress::Up => {
                    current_scene.borrow_mut().button_up().await;
                }
            },
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
    async fn button_down(&mut self) {
        lights::change(lights::LightChange::White(true)).await;
    }

    async fn button_up(&mut self) {
        lights::change(lights::LightChange::White(false)).await;
    }

    async fn long_press(&mut self) {
        enter(CurrentScene::Menu(MenuScene {
            current: MenuOption::Sniff,
            is_on: true,
        }))
        .await;
    }

    async fn tick(&mut self) {
        Timer::after_millis(2).await;
    }
}

#[derive(Clone, Debug)]
pub enum MenuOption {
    Bluetooth,
    Sleep,
    Erase,
    Sniff,
}

#[derive(Clone, Debug)]
pub struct MenuScene {
    pub current: MenuOption,
    is_on: bool,
}

impl Scene for MenuScene {
    async fn enter(&self) {
        lights::off().await;
    }

    async fn button_press(&mut self) {
        lights::off().await;

        match self.current {
            MenuOption::Sniff => {
                self.current = MenuOption::Erase;
            }
            MenuOption::Erase => {
                self.current = MenuOption::Sleep;
            }
            MenuOption::Sleep => {
                self.current = MenuOption::Bluetooth;
            }

            MenuOption::Bluetooth => self.current = MenuOption::Sniff,
        }
    }

    async fn tick(&mut self) {
        match self.current {
            MenuOption::Sniff => lights::change(lights::LightChange::White(self.is_on)).await,
            MenuOption::Sleep => lights::change(lights::LightChange::Green(self.is_on)).await,
            MenuOption::Erase => lights::change(lights::LightChange::Yellow(self.is_on)).await,
            MenuOption::Bluetooth => lights::change(lights::LightChange::Blue(self.is_on)).await,
        }

        self.is_on = !self.is_on;

        Timer::after_millis(400).await;
    }
}
