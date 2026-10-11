"""Independent TS 33.501 Annex A composition using Python's standard HMAC.
Inputs are TS 35.208 set 1 CK/IK/RAND/RES; these outputs are derived fixtures,
not a claim that 3GPP publishes this complete 5G vector.
"""
import hashlib, hmac, json
from pathlib import Path
def derive(key, fc, *params):
    s=bytes([fc])+b''.join(p+len(p).to_bytes(2,'big') for p in params)
    return hmac.new(key,s,hashlib.sha256).digest()
ck=bytes.fromhex('b40ba9a3c58b2a05bbf0d987b21bf8cb')
ik=bytes.fromhex('f769bcd751044604127672711c6d3441')
rand=bytes.fromhex('23553cbe9637a89d218ae64dae47bf35')
res=bytes.fromhex('a54211d5e3ba50bf')
sn=b'5G:mnc001.mcc001.3gppnetwork.org'
sqn=bytes.fromhex('55f328b43577')
ausf=derive(ck+ik,0x6a,sn,sqn)
seaf=derive(ausf,0x6c,sn)
amf=derive(seaf,0x6d,b'001010000000101',b'\0\0')
v={'kausf':ausf.hex(),'kseaf':seaf.hex(),'kamf':amf.hex(),'res_star':derive(ck+ik,0x6b,sn,rand,res)[16:].hex(),'knas_int':derive(amf,0x69,b'\2',b'\2')[16:].hex(),'knas_enc':derive(amf,0x69,b'\1',b'\2')[16:].hex()}
print(json.dumps(v,indent=2))
