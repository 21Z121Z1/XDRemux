"""Conservative interpretation of the owned simctl boot monitor's stdout."""
import hashlib
import re


def observe_boot_status(stdout, owned_udid):
    # simctl can exit zero after a terminal Data Migration Failed status.
    # This observed CLI format is not a readiness or Photos API guarantee.
    facts = {'state': 'blocked', 'owned_udid': owned_udid,
             'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
             'reason': 'boot monitor did not report a recognized successful terminal state'}
    try:
        text = stdout.decode('utf-8')
    except UnicodeDecodeError:
        return facts
    headers = re.findall(r'^Monitoring boot status for .+ \(([0-9A-Fa-f-]{36})\)\.$', text, re.MULTILINE)
    if len(headers) != 1 or headers[0].upper() != owned_udid:
        facts['reason'] = 'boot monitor output does not identify the owned simulator'
        return facts
    events = list(re.finditer(
        r'^\[[^\]\r\n]+\] Status=(\d+), isTerminal=(YES|NO), Elapsed=([0-9:]+)\.\r?\n[ \t]*([^\r\n]+)',
        text, re.MULTILINE))
    if not events:
        return facts
    final = events[-1]
    status, terminal, elapsed, message = final.groups()
    facts.update(status=int(status), terminal=terminal == 'YES', elapsed=elapsed, message=message.strip())
    terminals = [event for event in events if event[2] == 'YES']
    if (len(terminals) == 1 and terminals[0] is final and status == '4294967295'
            and message.strip() == 'Finished' and not text[final.end():].strip()):
        facts.update(state='completed')
        facts.pop('reason')
    elif status == '3' and terminal == 'YES':
        facts['reason'] = 'boot monitor reported Data Migration Failed (terminal status 3)'
    return facts
