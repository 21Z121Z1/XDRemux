from pathlib import Path
import hashlib,json,os,subprocess,sys,tempfile
root=Path('/workspace/XDRemux-integration');product_sha='8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a'
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
assert head==os.environ['XDREMUX_HARNESS_EXPECTED_SOURCE_SHA']
proof=Path(tempfile.mkdtemp(prefix='xdremux-control-entry-proof-'))
with tempfile.TemporaryDirectory(prefix='xdremux-control-entry-source-') as temporary:
 product=Path(temporary)/'product-source'
 subprocess.run(['git','clone','--shared','--no-checkout','--quiet','/workspace/XDRemux',str(product)],check=True,capture_output=True)
 subprocess.run(['git','-C',str(product),'checkout','--quiet','--detach',product_sha],check=True,capture_output=True)
 command=[sys.executable,str(root/'scripts/apple/pvt_native_import/readiness_control.py'),
          '--harness-repo',str(root),'--expected-harness-head',head,'--product-repo',str(product),
          '--expected-product-head',product_sha,'--output-root',str(proof/'control'),
          '--product-output-root',str(proof/'product')]
 result=subprocess.run(command,cwd=root,capture_output=True,timeout=60)
 (proof/'entry.stdout').write_bytes(result.stdout);(proof/'entry.stderr').write_bytes(result.stderr)
 assert result.returncode==2,result.stderr
 receipts=list((proof/'control').glob('control-*/result.json'));assert len(receipts)==1
 observed=json.loads(receipts[0].read_text());assert observed['status']=='experiment-blocked'
 assert observed['harness_source']['head']==head and observed['product_source']['head']==product_sha
 assert observed['product_probe_invocations']==0 and observed['commands']==[]
 assert len(observed['equivalence']['product_trees'])==7
 expected='a549744429ac698c0fa8e42bc9cee83677bc054c282315add984615bb6ff83bb'
 assert observed['product_probe_sha256']==expected
 assert observed['equivalence']['unchanged_original_probe_files']['scripts/apple/pvt_native_import/check_native_import.py']==expected
 manifest={'passed':True,'harness_source':head,'product_source':product_sha,'command':command,'exit_code':result.returncode,
           'meaning':'Actual CLI validated two separate exact clean checkouts and original probe/product equivalence, then refused native execution without its explicit flag. No Apple/CoreSimulator/Photos operation executed.',
           'receipt':str(receipts[0]),'probe_sha256':expected}
 (proof/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
assert not product.exists()
print('Retained control-entry proof: '+str(proof))
