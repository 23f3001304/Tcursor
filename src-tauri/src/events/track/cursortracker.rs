use crate::events::track::cursorlayer::CursorLayerBuilder;
use crate::events::track::cursortype::CursorType;

pub type CursorSamples = (Vec<(u32, CursorType)>, CursorLayerBuilder);
