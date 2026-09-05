"""Run the real native client against a local mapped-file control test peer."""
import json,mmap,pathlib,socket,struct,subprocess,sys,tempfile,time,unittest
TOOLS=pathlib.Path(__file__).parent.resolve();ROOT=TOOLS.parent;sys.path.insert(0,str(TOOLS))
from ap1_contract import compare_caller,GUARD
HEADER=struct.Struct('<IHHHHI16sQQQ')
def frame(kind,session,sequence,payload=b''):
    return HEADER.pack(0x3141504c,1,1,kind,0,len(payload),session,1,sequence,0)+payload
def exact(sock,n):
    b=b''
    while len(b)<n:
        part=sock.recv(n-len(b))
        if not part:raise EOFError('peer closed')
        b+=part
    return b
def receive(sock):
    h=HEADER.unpack(exact(sock,56));return h[3],h[6],h[8],exact(sock,h[5])
class NativeClientTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import os
        env=os.environ.copy();env['RUSTC']=subprocess.check_output(['rustup','which','--toolchain','stable','rustc'],text=True).strip()
        subprocess.run(['rustup','run','stable','cargo','build','--manifest-path',str(ROOT/'native-audio-client/Cargo.toml'),'--locked','--offline'],env=env,check=True,capture_output=True)
        cls.binary=ROOT/'native-audio-client/target/debug/ap1-native-client'
    def run_peer(self,fault=None):
        with tempfile.TemporaryDirectory() as tmp:
            path=pathlib.Path(tmp);session=bytes.fromhex('12'*16)
            with (path/'out').open('wb') as out:
                child=subprocess.Popen([str(self.binary),'--session-dir',tmp,'--session',session.hex()],stdout=out,stderr=subprocess.PIPE)
                try:
                    until=time.monotonic()+3
                    while not (path/'ap1.control').exists():
                        if child.poll() is not None:raise AssertionError(child.stderr.read())
                        self.assertLess(time.monotonic(),until);time.sleep(.01)
                    config=(path/'ap1.control').read_bytes();port=struct.unpack_from('<H',config)[0]
                    with (path/'ap1.audio').open('r+b') as backing,mmap.mmap(backing.fileno(),0) as shared,socket.create_connection(('127.0.0.1',port),timeout=7) as sock:
                        witness=struct.unpack_from('<Q',shared,32)[0]^0x8d396b274e105ac3;struct.pack_into('<Q',shared,40,witness)
                        # Fragmented Hello and coalescible stream are real socket I/O.
                        hello=frame(1,session,0,config[20:52]+struct.pack('<II',256,4192))
                        for b in hello:sock.sendall(bytes([b]))
                        self.assertEqual(receive(sock),(1,session,0,b''));self.assertEqual(struct.unpack_from('<Q',shared,56)[0],witness^1)
                        sock.sendall(frame(2,session,0));requests=0
                        while True:
                            kind,got,seq,payload=receive(sock);self.assertEqual(got,session)
                            if kind==5:
                                sock.sendall(frame(6,session,seq));break
                            self.assertEqual(kind,3);requests+=1
                            n,inp,outoff,stride,gain,silence,reserved=struct.unpack('<IIIIdII',payload)
                            self.assertEqual((inp,outoff,stride,reserved),(64,2128,1032,0))
                            for ch in range(2):
                                for i in range(n):
                                    value=struct.unpack_from('<f',shared,inp+ch*stride+4*(i+1))[0]
                                    struct.pack_into('<f',shared,outoff+ch*stride+4*(i+1),value*gain)
                            output_silence=3 if silence==3 or gain==0 else 0
                            if silence & 1:
                                self.assertTrue(all(struct.unpack_from('<f',shared,inp+4*(i+1))[0]==0 for i in range(n)))
                            if requests==10:
                                self.assertEqual(silence,1);self.assertEqual(output_silence,0)
                                self.assertNotEqual(struct.unpack_from('<f',shared,inp+stride+4)[0],0.)
                            if requests==9:
                                self.assertEqual((silence,gain,output_silence),(0,0.,3))
                                self.assertNotEqual(struct.unpack_from('<f',shared,inp+4)[0],0.)
                            if fault=='invalid_mask':output_silence=1<<32
                            if fault=='false_silence':output_silence=3
                            if fault=='corrupt':struct.pack_into('<f',shared,outoff+4,99.)
                            if fault=='disconnect':break
                            if fault=='timeout':time.sleep(5.2);break
                            sock.sendall(frame(4,session,seq+(1 if fault=='stale' else 0),struct.pack('<IIQ',n,outoff,output_silence)))
                            if fault:
                                self.assertEqual(sock.recv(1),b'','failed response must never replay or reuse');break
                    child.wait(timeout=3)
                    records=[json.loads(line) for line in (path/'out').read_bytes().splitlines()]
                    return child.returncode,records,requests
                finally:
                    if child.poll() is None:child.kill();child.wait()
                    child.stderr.close()
    def test_real_mapping_roundtrip_and_independent_admission(self):
        code,records,n=self.run_peer();self.assertEqual((code,n),(0,10))
        result=compare_caller({'raw_exit':code,'records':records,'cleanup':{'owned_descendants_zero':True,'process_group_empty':True}})
        self.assertEqual(result['samples_compared'],1502);self.assertEqual(result['maximum_absolute_error'],0)
    def test_bad_samples_retained_before_rejection(self):
        code,r,n=self.run_peer('corrupt');self.assertEqual((code,n),(1,1));self.assertEqual(r[-1]['event'],'ap1_client_error')
        self.assertEqual(r[-2]['event'],'ap1_client_block');self.assertEqual(r[-2]['output_bits'][0][1],struct.unpack('<I',struct.pack('<f',99.))[0])
    def test_invalid_output_silence_claims_are_retained_and_stop(self):
        for fault,flags in [('invalid_mask',1<<32),('false_silence',3)]:
            with self.subTest(fault=fault):
                code,records,n=self.run_peer(fault)
                self.assertEqual((code,n),(1,1))
                self.assertEqual(records[-2]['output_silence_flags'],flags)
                self.assertEqual(records[-1]['event'],'ap1_client_error')
                self.assertIn('silence',records[-1]['detail'])
    def test_missing_stale_disconnected_responses_never_replay(self):
        for fault in ('stale','disconnect','timeout'):
            with self.subTest(fault=fault):
                code,r,n=self.run_peer(fault);self.assertEqual((code,n),(1,1));self.assertEqual([x['event'] for x in r],['ap1_client_ready','ap1_client_error'])
if __name__=='__main__':unittest.main()
