from pathlib import Path
import datetime
import hashlib
import json
import platform
import resource
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parent
BUILD = json.loads((ROOT/'build-receipt.json').read_text())
FIXTURE = ROOT/'baseline-source/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic'
EXPECTED_FIXTURE = '32a2c5df1d3ca14e64d24ad291bc68067edcaaec6a7f9fc760d331bbd6576cbc'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(arm, mode, source, output, label):
    command = [BUILD['arms'][arm]['binary'],mode,str(source),str(output)]
    cpu_before=resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter()
    with (ROOT/(label+'.stdout.jsonl')).open('w') as stdout, (ROOT/(label+'.stderr.log')).open('w') as stderr:
        result = subprocess.run(command,cwd=ROOT,stdout=stdout,stderr=stderr,timeout=120)
    cpu_after=resource.getrusage(resource.RUSAGE_CHILDREN)
    row = {'arm':arm,'mode':mode,'command':command,'returncode':result.returncode,'process_wall_seconds':time.perf_counter()-started,
           'process_user_seconds':cpu_after.ru_utime-cpu_before.ru_utime,
           'process_sys_seconds':cpu_after.ru_stime-cpu_before.ru_stime,
           'children_maxrss_high_water_kib':cpu_after.ru_maxrss}
    REPORT['commands'].append(row)
    save()
    result.check_returncode()
    return [json.loads(line) for line in (ROOT/(label+'.stdout.jsonl')).read_text().splitlines()]

def save():
    (ROOT/'comparison-report.json').write_text(json.dumps(REPORT,indent=2)+'\n')

def byte_compare(left,right,label):
    a,b=left.read_bytes(),right.read_bytes()
    row={'comparison':label,'left':str(left.relative_to(ROOT)),'right':str(right.relative_to(ROOT)),
         'left_bytes':len(a),'right_bytes':len(b),'equal_bytes':a==b,
         'left_sha256':hashlib.sha256(a).hexdigest(),'right_sha256':hashlib.sha256(b).hexdigest()}
    if a!=b:
        row['first_difference']=next((i for i,(x,y) in enumerate(zip(a,b)) if x!=y),min(len(a),len(b)))
    REPORT['byte_comparisons'].append(row)
    save()
    assert a==b, row

assert digest(FIXTURE)==EXPECTED_FIXTURE
REPORT={'status':'running','captured_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'host':platform.uname()._asdict(),'build_receipt_sha256':digest(ROOT/'build-receipt.json'),
        'fixture_sha256':digest(FIXTURE),'commands':[],'byte_comparisons':[],'samples':[],
        'limits':['Linux x86_64 kernel only; not Apple/native/E2E evidence.',
                  'Public primary was decoded and linearly area-resized; recovery target is synthetic.',
                  'Timed region is exactly the public local-fit call. Input loading and output writes are outside it.',
                  'Every measured call uses a fresh process; same prepared request path is shared by both arms.',
                  'Absolute performance budgets and integration producer remain untouched.']}
save()
try:
    for arm in ('baseline','candidate'):
        rows=run(arm,'prepare',FIXTURE,ROOT/(arm+'-request.bin'),arm+'-prepare')
        REPORT[arm+'_preparation']=rows
    byte_compare(ROOT/'baseline-request.bin',ROOT/'candidate-request.bin','independent preparation')
    request=ROOT/'baseline-request.bin'
    for arm in ('baseline','candidate'):
        rows=run(arm,'cases',request,ROOT/(arm+'-cases'),arm+'-cases')
        REPORT[arm+'_cases']=rows
    left={p.name:p for p in (ROOT/'baseline-cases').iterdir()}
    right={p.name:p for p in (ROOT/'candidate-cases').iterdir()}
    assert left.keys()==right.keys()
    for name in sorted(left):
        byte_compare(left[name],right[name],name)
    order=[(True,0,'baseline'),(True,0,'candidate')]
    for iteration in range(7):
        arms=('baseline','candidate') if iteration%2==0 else ('candidate','baseline')
        order.extend((False,iteration,arm) for arm in arms)
    reference=ROOT/'baseline-cases/public-primary-synthetic-target.bin'
    for ordinal,(warmup,iteration,arm) in enumerate(order):
        label=f'{ordinal:02d}-{arm}-'+('warmup' if warmup else str(iteration))
        output=ROOT/(label+'-output')
        rows=run(arm,'fit',request,output,label)
        assert len(rows)==1 and rows[0]['status']=='ok'
        row={'ordinal':ordinal,'arm':arm,'warmup':warmup,'iteration':iteration,**rows[0]}
        REPORT['samples'].append(row)
        byte_compare(reference,output/'public-primary-synthetic-target.bin',label)
        print(json.dumps(row),flush=True)
    samples=REPORT['samples']
    medians={arm:statistics.median(r['wall_seconds'] for r in samples if r['arm']==arm and not r['warmup']) for arm in ('baseline','candidate')}
    pairs=[]
    for iteration in range(7):
        pair={r['arm']:r['wall_seconds'] for r in samples if not r['warmup'] and r['iteration']==iteration}
        pairs.append({'iteration':iteration,**pair,'candidate_over_baseline':pair['candidate']/pair['baseline']})
    REPORT['summary']={'median_seconds':medians,'candidate_over_baseline_median':medians['candidate']/medians['baseline'],
                       'pairs':pairs,'median_paired_ratio':statistics.median(r['candidate_over_baseline'] for r in pairs)}
    assert digest(FIXTURE)==EXPECTED_FIXTURE
    REPORT['fixture_unchanged']=True
    REPORT['status']='completed'
    print(json.dumps(REPORT['summary'],indent=2),flush=True)
except BaseException as error:
    REPORT['status']='failed'
    REPORT['error']=str(error)
    raise
finally:
    save()
