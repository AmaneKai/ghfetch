pub enum Asset {
    Pfp,
    CarlOsDemo,
    PythonInterpreter,
    RustTetris,
    Sokobot,
    ValentinesTui,
    FirstRice,
    HyprlandArch,
    I3Rice,
    Cv,
}

impl Asset {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Pfp               => "/public/screenshots/pfp.jpg",
            Self::CarlOsDemo        => "/public/gifs/CarlOS-demo.webm",
            Self::PythonInterpreter => "/public/gifs/python-intep.webm",
            Self::RustTetris        => "/public/gifs/tetris-demo.webm",
            Self::Sokobot           => "/public/gifs/sokobot.webm",
            Self::ValentinesTui     => "/public/gifs/valentines.webm",
            Self::FirstRice         => "/public/screenshots/firstrice.webp",
            Self::HyprlandArch      => "/public/screenshots/hyprlandarch.webp",
            Self::I3Rice            => "/public/screenshots/i3rice.png",
            Self::Cv                => "/public/CV.pdf",
        }
    }
}
