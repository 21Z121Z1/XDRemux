from pathlib import Path

path = Path("crates/xdremux-runtime/tests/apple_adapter.rs")
text = path.read_text()
old = '''    for (role, matte) in [
        (AppleSemanticRole::Skin, &preflight.skin_matte),
        (AppleSemanticRole::Hair, &preflight.hair_matte),
        (AppleSemanticRole::Teeth, &preflight.teeth_matte),
        (AppleSemanticRole::Glasses, &preflight.glasses_matte),
    ] {
'''
new = '''    for (role, matte) in [
        (
            AppleSemanticRole::Skin,
            preflight
                .skin_matte
                .as_ref()
                .expect("complete profile must contain skin matte"),
        ),
        (AppleSemanticRole::Hair, &preflight.hair_matte),
        (
            AppleSemanticRole::Teeth,
            preflight
                .teeth_matte
                .as_ref()
                .expect("complete profile must contain teeth matte"),
        ),
        (
            AppleSemanticRole::Glasses,
            preflight
                .glasses_matte
                .as_ref()
                .expect("complete profile must contain glasses matte"),
        ),
    ] {
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"complete Portrait semantic regression expected one match, found {count}")
path.write_text(text.replace(old, new, 1))
print("complete Portrait regression adapted to optional complete-only mattes")
