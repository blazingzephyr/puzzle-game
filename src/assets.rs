
use bevy::asset::Handle;
use bevy::ecs::system::Resource;
use bevy::render::texture::Image;
use bevy::sprite::TextureAtlas;
use bevy::text::Font;
use bevy_asset_loader::asset_collection::AssetCollection;

#[derive(AssetCollection, Resource)]
pub struct MenuAssets
{    
    #[asset(key = "loading_icon")]
    pub loading_icon: Handle<TextureAtlas>,

    #[asset(key = "loading_font")]
    pub loading_font: Handle<Font>,

    #[asset(key = "app_icon")]
    pub app_icon: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
pub struct GameAssets
{
    #[asset(key = "menu_bg")]
    pub menu_bg: Handle<Image>,

    #[asset(key = "game_over_bg")]
    pub game_over_bg: Handle<Image>,

    #[asset(key = "full_completion_bg")]
    pub full_completion_bg: Handle<Image>,

    #[asset(key = "main_font")]
    pub main_font: Handle<Font>,

    #[asset(key = "sonic")]
    pub sonic: Handle<TextureAtlas>,

    #[asset(key = "level1")]
    pub level1: Handle<Image>,

    #[asset(key = "level2")]
    pub level2: Handle<Image>,
}
