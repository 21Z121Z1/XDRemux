from pathlib import Path

path = Path("scripts/prototype_oppo_native_portrait_c.py")
text = path.read_text()


def replace_last(label: str, old: str, new: str) -> None:
    global text
    index = text.rfind(old)
    if index < 0:
        raise SystemExit(f"{label}: pattern not found")
    if text.find(old, index + 1) >= 0:
        raise SystemExit(f"{label}: internal last-match invariant failed")
    text = text[:index] + new + text[index + len(old):]


replace_last(
    "auxiliary payload splice tail",
    '''#[cfg(any(target_os = "macos", test))]
fn producer_focus_region''',
    "''',",
)
replace_last(
    "Vision branch splice tail",
    '''    let simulated_aperture = resolve_simulated_aperture(
''',
    "''',",
)
replace_last(
    "CLI product splice tail",
    '''    }
}

#[derive(Debug, Args)]
struct ConvertArgs''',
    '''    }
''',''',
)

compile(text, str(path), "exec")
path.write_text(text)
print("prototype splice ownership fixed")
