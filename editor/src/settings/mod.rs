use crate::{
    inspector::editors::make_property_editors_container,
    message::MessageSender,
    settings::{
        camera::CameraSettings, debugging::DebuggingSettings, graphics::GraphicsSettings,
        keys::KeyBindings, model::ModelSettings, move_mode::MoveInteractionModeSettings,
        navmesh::NavmeshSettings, recent::RecentFiles, rotate_mode::RotateInteractionModeSettings,
        selection::SelectionSettings, windows::WindowsSettings,
    },
    Engine, MSG_SYNC_FLAG,
};
use fyrox::{
    core::{log::Log, pool::Handle, reflect::prelude::*, scope_profile},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                enumeration::EnumPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                key::{HotKeyPropertyEditorDefinition, KeyBindingPropertyEditorDefinition},
                PropertyEditorDefinitionContainer,
            },
            InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    renderer::{CsmSettings, QualitySettings, ShadowMapPrecision},
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, rc::Rc};

pub mod camera;
pub mod debugging;
pub mod graphics;
pub mod keys;
pub mod model;
pub mod move_mode;
pub mod navmesh;
pub mod recent;
pub mod rotate_mode;
pub mod selection;
pub mod windows;

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default, Debug, Reflect)]
pub struct Settings {
    pub selection: SelectionSettings,
    pub graphics: GraphicsSettings,
    pub debugging: DebuggingSettings,
    pub move_mode_settings: MoveInteractionModeSettings,
    pub rotate_mode_settings: RotateInteractionModeSettings,
    pub model: ModelSettings,
    pub camera: CameraSettings,
    pub navmesh: NavmeshSettings,
    pub key_bindings: KeyBindings,
    #[reflect(hidden)]
    pub recent: RecentFiles,
    #[serde(default)]
    #[reflect(hidden)]
    pub windows: WindowsSettings,
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    RonSpanned(ron::error::SpannedError),
    Ron(ron::Error),
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ron::error::SpannedError> for SettingsError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::RonSpanned(e)
    }
}

impl From<ron::Error> for SettingsError {
    fn from(e: ron::Error) -> Self {
        Self::Ron(e)
    }
}

impl Settings {
    const FILE_NAME: &'static str = "settings.ron";

    fn full_path() -> PathBuf {
        Self::FILE_NAME.into()
    }

    pub fn load() -> Result<Self, SettingsError> {
        let file = File::open(Self::full_path())?;
        Ok(ron::de::from_reader(file)?)
    }

    pub fn save(&mut self) -> Result<(), SettingsError> {
        let file = File::create(Self::full_path())?;
        self.recent.deduplicate_and_refresh();
        ron::ser::to_writer_pretty(file, self, PrettyConfig::default())?;
        Ok(())
    }

    fn make_property_editors_container(
        sender: MessageSender,
    ) -> Rc<PropertyEditorDefinitionContainer> {
        let container = make_property_editors_container(sender);

        container.insert(InspectablePropertyEditorDefinition::<GraphicsSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<SelectionSettings>::new());
        container.insert(EnumPropertyEditorDefinition::<ShadowMapPrecision>::new());
        container.insert(InspectablePropertyEditorDefinition::<DebuggingSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<CsmSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<QualitySettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<CameraSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<
            MoveInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<
            RotateInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<ModelSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<NavmeshSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<KeyBindings>::new());
        container.insert(HotKeyPropertyEditorDefinition);
        container.insert(KeyBindingPropertyEditorDefinition);

        Rc::new(container)
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        PropertyAction::from_field_kind(&property_changed.value).apply(
            &property_changed.path(),
            self,
            &mut Log::verify,
        );
    }
}

impl SettingsWindow {
    pub fn new(engine: &mut Engine) -> Self {
        let ok;
        let default;

        let ctx = &mut engine.user_interface.build_ctx();

        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::Text("Settings".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(2.0))
                                    .on_row(0),
                            )
                            .with_content(inspector)
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        default = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(80.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Default")
                                        .build(ctx);
                                        default
                                    })
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(80.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            ok,
            default,
            inspector,
        }
    }

    pub fn open(&self, ui: &mut UserInterface, settings: &Settings, sender: &MessageSender) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        self.sync_to_model(ui, settings, sender);
    }

    fn sync_to_model(&self, ui: &mut UserInterface, settings: &Settings, sender: &MessageSender) {
        let context = InspectorContext::from_object(
            settings,
            &mut ui.build_ctx(),
            Settings::make_property_editors_container(sender.clone()),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
        );
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        settings: &mut Settings,
        sender: &MessageSender,
    ) {
        scope_profile!();

        let mut need_save = false;

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                engine.user_interface.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.default {
                *settings = Default::default();
                need_save = true;
                self.sync_to_model(&mut engine.user_interface, settings, sender);
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                settings.handle_property_changed(property_changed);
                need_save = true;
            }
        }

        let graphics_context = engine.graphics_context.as_initialized_mut();

        if need_save {
            if settings.graphics.quality != graphics_context.renderer.get_quality_settings() {
                if let Err(e) = graphics_context
                    .renderer
                    .set_quality_settings(&settings.graphics.quality)
                {
                    Log::err(format!(
                        "An error occurred at attempt to set new graphics settings: {:?}",
                        e
                    ));
                } else {
                    Log::info("New graphics quality settings were successfully set!");
                }
            }

            // Save config
            match settings.save() {
                Ok(_) => {
                    Log::info("Settings were successfully saved!");
                }
                Err(e) => {
                    Log::err(format!("Unable to save settings! Reason: {:?}!", e));
                }
            };
        }
    }
}
