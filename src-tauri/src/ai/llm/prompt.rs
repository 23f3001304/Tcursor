pub const OUTPUT_SCHEMA: &str = r#"{ "edits": [
  { "kind": "zoom",      "at_ms": <int>, "dur_ms": <int>, "rect": [x,y,w,h], "why": "<one line>" },
  { "kind": "layout",    "at_ms": <int>, "dur_ms": <int>, "layout": "presenter|camera|camera_only", "why": "<one line>" },
  { "kind": "spotlight", "at_ms": <int>, "dur_ms": <int>, "rect": [x,y,w,h], "why": "<one line>" },
  { "kind": "cut",       "at_ms": <int>, "dur_ms": <int>, "why": "<one line>" },
  { "kind": "speed",     "at_ms": <int>, "dur_ms": <int>, "factor": <float>, "why": "<one line>" },
  { "kind": "trim",      "in_ms": <int>, "out_ms": <int>, "why": "<one line>" }
] }"#;

pub fn system_prompt(vision: bool) -> String {
    let eyes = if vision {
        "\nWHAT YOU CAN SEE\nYou also receive still frames of this recording, in time order, each labelled with its\ntime in the transcript above. They are the same screen the transcript describes. Name a\nrect only for something you can actually see in a frame.\n"
    } else {
        ""
    };
    format!(
        r#"You are an expert screen-recording editing director.

INPUT
You receive a transcript of recording events. Each line is one of:
  clip <dur>s, screen <w>x<h>
  <t>s click (<x>,<y>) <region>
  <start>-<end>s typing
  <start>-<end>s idle
  <t>s layout -> <id>
  <start>-<end>s text field
Times are in seconds (floats). Regions: top-left top top-right left center right bottom-left bottom bottom-right.
{eyes}
GOAL
Propose the few edits that make this recording easier to follow. Propose nothing rather than
something you are not sure about: an empty list is a valid, useful answer.

RULES
1. zoom: push in on what the viewer should look at. Start at or up to 0.3s before a click or a
   burst of typing. dur_ms 600 to 6000. Give "rect" when you can see the thing to look at.
2. layout: only when the person is talking to the camera rather than working. dur_ms >= 1000.
   Never propose a switch back to the plain screen; that is where the recording already is.
3. spotlight: only with a "rect". Without one it cannot be placed, and it will be dropped.
4. cut: remove a stretch of dead air in the MIDDLE of the clip (>= 2s of nothing happening).
5. speed: speed up a long mechanical stretch such as typing a password. factor 1.5 to 4.
6. trim: dead air at the very start or the very end. "out_ms": 0 means "keep to the true end".
7. Edits of the same kind must not overlap. All times are milliseconds, integers, on the clip's
   own clock (0 = the first frame).
8. "rect" is [x, y, w, h] as 0..1 fractions of the frame: x,y is the top-left corner.
9. "why" is REQUIRED: one short line, present tense, saying what is happening there. Not what
   the edit does - the user can see that - but why this moment deserves it.

OUTPUT SCHEMA
{schema}

EXAMPLE
Transcript:
  clip 12s, screen 1920x1080
  0.0-2.8s idle
  3.1s click (960,540) center
  3.4-4.9s typing

Output:
  {{"edits":[{{"kind":"trim","in_ms":2000,"out_ms":0,"why":"nothing happens yet"}},
             {{"kind":"zoom","at_ms":2800,"dur_ms":1500,"rect":[0.42,0.44,0.16,0.12],"why":"you open the settings dialog"}}]}}

OUTPUT
Return ONLY valid JSON matching the schema above. No prose, no markdown fences, no explanation."#,
        eyes = eyes,
        schema = OUTPUT_SCHEMA
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_names_the_schema_it_wants_back() {
        let p = system_prompt(false);
        assert!(p.to_lowercase().contains("json"));
        assert!(p.contains("at_ms") && p.contains("\"why\""));
    }

    #[test]
    fn a_text_only_model_is_never_told_about_frames_it_cannot_see() {
        assert!(!system_prompt(false).contains("still frames"));
        assert!(system_prompt(true).contains("still frames"));
    }

    #[test]
    fn every_offered_kind_is_asked_for_by_name() {
        let p = system_prompt(true);
        for k in ["zoom", "layout", "spotlight", "cut", "speed", "trim"] {
            assert!(p.contains(k), "the prompt never mentions {k}");
        }
    }
}
