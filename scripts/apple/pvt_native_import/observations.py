"""Stage facts for the owned PVT probe, independent of Photos admission."""
import math
import re
import json
import time


REPRESENTATIONS = ('share-control', 'share-url', 'share')
UI_PHASES = ('test_runner', 'test_setup', 'test_body', *REPRESENTATIONS, 'save_callback')


def check_reader_observation(report, *, source_head, session_id, phase):
    observed = report.get('observation', {})
    elapsed = observed.get('elapsed_seconds')
    if (observed.get('source_head') != source_head or observed.get('session_id') != session_id
            or observed.get('phase') != phase or observed.get('clock') != 'reader-process-monotonic'
            or type(elapsed) not in (int, float) or not math.isfinite(elapsed) or elapsed < 0):
        raise ValueError('native reader report source/session/monotonic observation mismatch')


class PVTStages:
    def __init__(self, source_head, session_id, phases, *, clock=time.monotonic):
        self.clock, self.started = clock, clock()
        self.facts = {'schema_version': 1, 'source_head': source_head, 'session_id': session_id,
                      'phases': {phase: {'state': 'not_started'} for phase in phases},
                      'events': [], 'last_observed_boundary': None, 'admission': 'unobserved'}

    def record(self, phase, state, boundary, *, elapsed=None, clock='driver-monotonic', **facts):
        event = {'phase': phase, 'state': state, 'boundary': boundary,
                 'elapsed_seconds': self.clock() - self.started if elapsed is None else elapsed,
                 'clock': clock, **facts}
        self.facts['events'].append(event)
        self.facts['phases'][phase] = {'state': state, 'last_event': event}
        self.facts['last_observed_boundary'] = event

    def stop_active(self, *, timed_out=False):
        # A timeout is a limit on observation; it is not a native rejection.
        state = 'unknown_after_timeout' if timed_out else 'blocked'
        for phase, facts in self.facts['phases'].items():
            if facts['state'] in ('started', 'observed'):
                facts['state'] = state
        self.facts['missing_required_stages'] = [phase for phase, facts in self.facts['phases'].items()
                                                if facts['state'] != 'completed']


def parse_share_observations(text, *, source_head, session_id, outcome=None):
    """Read native events strictly; retain older logs without inventing clocks."""
    trace = PVTStages(source_head, session_id, UI_PHASES)
    facts = trace.facts
    structured = 'PVT_STAGE_EVENT:' in text
    facts.update(format='structured-events' if structured else 'legacy-text',
                 monotonic_boundaries_available=structured)
    previous_time = -1
    boundaries = {}
    sheets = {}
    current = None

    if structured:
        for line in text.splitlines():
            if 'PVT_STAGE_EVENT:' not in line:
                continue
            event = json.loads(line.split('PVT_STAGE_EVENT:', 1)[1])
            if (not isinstance(event, dict) or event.get('schema_version') != 1
                    or event.get('source_head') != source_head or event.get('session_id') != session_id):
                raise ValueError('PVT event source/session binding mismatch')
            elapsed = event.get('elapsed_seconds')
            if type(elapsed) not in (int, float) or not math.isfinite(elapsed) or elapsed < previous_time or elapsed < 0:
                raise ValueError('PVT event has invalid/decreasing monotonic time')
            phase, boundary = event.get('phase'), event.get('boundary')
            prior = boundaries.get(phase)
            valid = False
            state = 'observed'
            if phase == 'test_setup':
                valid = (prior, boundary) in ((None, 'started'), ('started', 'completed'))
                state = boundary
            elif phase == 'test_body':
                valid = ((prior is None and boundary == 'started' and boundaries.get('test_setup') == 'completed')
                         or (prior == 'started' and boundary == 'completed' and boundaries.get('save_callback') == 'completed')
                         or (prior == 'started' and boundary == 'blocked'
                             and all(boundaries.get(p) == 'completed' for p in REPRESENTATIONS)
                             and all(sheets.get(p) == 0 for p in REPRESENTATIONS[1:])))
                state = boundary
            elif phase in REPRESENTATIONS:
                if boundary == 'launch_started':
                    index = REPRESENTATIONS.index(phase)
                    valid = (prior is None and boundaries.get('test_body') == 'started'
                             and all(boundaries.get(p) == 'completed' for p in REPRESENTATIONS[:index])
                             and 'save_callback' not in boundaries)
                    current, state = phase, 'started'
                elif boundary == 'app_ready':
                    valid = prior == 'launch_started' and current == phase
                elif boundary == 'sheet_observed':
                    count = event.get('save_candidates')
                    valid = prior == 'app_ready' and type(count) is int and count >= 0 and current == phase
                    sheets[phase] = count
                elif boundary == 'completed':
                    valid = (prior == 'sheet_observed' and current == phase
                             and (phase == 'share-control' or sheets.get(phase) == 0
                                  or boundaries.get('save_callback') == 'completed'))
                    state = 'completed'
            elif phase == 'save_callback':
                valid = ((prior is None and boundary == 'selected' and current in REPRESENTATIONS[1:]
                          and boundaries.get(current) == 'sheet_observed' and sheets.get(current) == 1)
                         or (prior == 'selected' and boundary == 'completed'))
                state = 'started' if boundary == 'selected' else 'completed'
            if not valid:
                raise ValueError('duplicate, out-of-order or contradictory PVT event: ' + str((phase, boundary)))
            trace.record(phase, state, boundary, elapsed=elapsed, clock='test-process-monotonic',
                         **({'save_candidates': event['save_candidates']} if boundary == 'sheet_observed' else {}))
            boundaries[phase], previous_time = boundary, elapsed
    else:
        # Archived receipts predate stage clocks. Only their explicit sheet
        # observations are retained; launch/setup/callback progress is unknown.
        for phase, count in re.findall(r'PVT_SHARE_UI_OBSERVATION: phase=(share(?:-control|-url)?) save_candidates=(\d+)', text):
            if phase in sheets or REPRESENTATIONS.index(phase) != len(sheets):
                raise ValueError('duplicate or out-of-order legacy PVT sheet observation')
            sheets[phase] = int(count)
            trace.record(phase, 'observed', 'sheet_observed', elapsed=0, clock='unavailable', save_candidates=int(count))
            facts['events'][-1]['elapsed_seconds'] = None
    facts['ui_observations'] = [{'phase': phase, 'save_candidates': count} for phase, count in sheets.items()]
    facts['missing_representation_observations'] = [phase for phase in REPRESENTATIONS if phase not in sheets]
    facts['representation_coverage_complete'] = not facts['missing_representation_observations']
    trace.stop_active(timed_out=outcome == 'uncertain-after-timeout')
    return facts
