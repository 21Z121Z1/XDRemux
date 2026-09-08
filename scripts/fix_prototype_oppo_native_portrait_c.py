from pathlib import Path

path = Path("scripts/prototype_oppo_native_portrait_c.py")
text = path.read_text()
old = "    write(path, text[:start_index] + replacement + text[end_index:])"
new = "    write(path, text[:start_index] + replacement + text[end_index + len(end):])"
count = text.count(old)
if count != 1:
    raise SystemExit(f"splice helper replacement expected one match, found {count}")
text = text.replace(old, new, 1)
compile(text, str(path), "exec")
path.write_text(text)
print("prototype splice helper now consumes the end marker")
