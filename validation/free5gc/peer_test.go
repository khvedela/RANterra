package main

// Independently authored test peer. No engine keys, plaintext replies or state are
// consulted: all responses are derived from received wire PDUs and public lab credentials.
import (
	"bufio"
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"github.com/free5gc/aper"
	"github.com/free5gc/nas"
	"github.com/free5gc/nas/security"
	"github.com/free5gc/ngap"
	n "github.com/free5gc/ngap/ngapType"
	"github.com/free5gc/util/milenage"
	"os"
	"os/exec"
	"testing"
	"time"
)

type peerHarness struct {
	t    *testing.T
	send *json.Encoder
	recv *bufio.Scanner
	now  uint64
}
type reply struct {
	Bytes      []byte
	Error      string
	Registered bool
	Raw        map[string]any
}

func endpoint(t *testing.T) *peerHarness {
	t.Helper()
	path := os.Getenv("RANTERRA_INTEROP_ENDPOINT")
	if path == "" {
		t.Fatal("RANTERRA_INTEROP_ENDPOINT must name the built Rust adapter")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	t.Cleanup(cancel)
	cmd := exec.CommandContext(ctx, path)
	input, e := cmd.StdinPipe()
	must(t, e)
	output, e := cmd.StdoutPipe()
	must(t, e)
	cmd.Stderr = os.Stderr
	must(t, cmd.Start())
	t.Cleanup(func() { input.Close(); must(t, cmd.Wait()) })
	scan := bufio.NewScanner(output)
	scan.Buffer(make([]byte, 4096), 300000)
	return &peerHarness{t: t, send: json.NewEncoder(input), recv: scan}
}
func (h *peerHarness) call(op string, id uint32, b []byte, ul bool) reply {
	h.t.Helper() // Rust Vec<u8> is an integer array, not Go's base64 []byte JSON convention.
	ints := make([]int, len(b))
	for i, v := range b {
		ints[i] = int(v)
	}
	return h.command(map[string]any{"op": op, "id": id, "bytes": ints, "uplink": ul, "now": h.now})
}
func (h *peerHarness) command(c map[string]any) reply {
	must(h.t, h.send.Encode(c))
	if !h.recv.Scan() {
		h.t.Fatalf("Rust adapter stopped: %v", h.recv.Err())
	}
	var r struct {
		Bytes      []int
		Error      string
		Registered bool
	}
	must(h.t, json.Unmarshal(h.recv.Bytes(), &r))
	wire := make([]byte, len(r.Bytes))
	for i, v := range r.Bytes {
		wire[i] = byte(v)
	}
	var raw map[string]any
	must(h.t, json.Unmarshal(h.recv.Bytes(), &raw))
	return reply{wire, r.Error, r.Registered, raw}
}
func (h *peerHarness) ok(op string, id uint32, b []byte, ul bool) reply {
	h.t.Helper()
	r := h.call(op, id, b, ul)
	if r.Error != "" {
		h.t.Fatalf("%s: %s", op, r.Error)
	}
	return r
}
func must(t *testing.T, e error) {
	t.Helper()
	if e != nil {
		t.Fatal(e)
	}
}
func check(t *testing.T, yes bool, why string) {
	t.Helper()
	if !yes {
		t.Fatal(why)
	}
}
func unhex(s string) []byte {
	b, e := hex.DecodeString(s)
	if e != nil {
		panic(e)
	}
	return b
}
func plain(t *testing.T, b []byte) *nas.Message {
	t.Helper()
	m := nas.NewMessage()
	data := bytes.Clone(b)
	must(t, m.PlainNasDecode(&data))
	return m
}
func canonical(t *testing.T, b []byte) []byte {
	t.Helper()
	out, e := plain(t, b).PlainNasEncode()
	must(t, e)
	return out
}
func registration(id uint32) []byte {
	// TS 24.501: initial, no key available, null-scheme SUCI, 001/01, route 0, public key 0.
	b := unhex("7e004171000d0100f110f0ff0000")
	digits := fmt.Sprintf("%010d", id)
	for i := 0; i < 10; i += 2 {
		b = append(b, (digits[i]-'0')|((digits[i+1]-'0')<<4))
	}
	return append(b, 0x2e, 2, 0xa0, 0x20)
}
func envelope(t *testing.T, id uint32, b []byte, initial, ul bool) []byte {
	t.Helper()
	p := n.NGAPPDU{Present: n.NGAPPDUPresentInitiatingMessage, InitiatingMessage: &n.InitiatingMessage{Criticality: n.Criticality{Value: n.CriticalityPresentIgnore}}}
	m := p.InitiatingMessage
	loc := &n.UserLocationInformation{Present: n.UserLocationInformationPresentUserLocationInformationNR, UserLocationInformationNR: &n.UserLocationInformationNR{NRCGI: n.NRCGI{PLMNIdentity: n.PLMNIdentity{Value: []byte{0, 0xf1, 0x10}}, NRCellIdentity: n.NRCellIdentity{Value: aper.BitString{Bytes: []byte{0, 0, 0, 0, 0x10}, BitLength: 36}}}, TAI: n.TAI{PLMNIdentity: n.PLMNIdentity{Value: []byte{0, 0xf1, 0x10}}, TAC: n.TAC{Value: []byte{0, 0, 1}}}}}
	if initial {
		m.ProcedureCode.Value = 15
		m.Value.Present = n.InitiatingMessagePresentInitialUEMessage
		m.Value.InitialUEMessage = &n.InitialUEMessage{}
		m.Value.InitialUEMessage.ProtocolIEs.List = []n.InitialUEMessageIEs{
			{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 0}, Value: n.InitialUEMessageIEsValue{Present: n.InitialUEMessageIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
			{Id: n.ProtocolIEID{Value: 38}, Criticality: n.Criticality{Value: 0}, Value: n.InitialUEMessageIEsValue{Present: n.InitialUEMessageIEsPresentNASPDU, NASPDU: &n.NASPDU{Value: b}}},
			{Id: n.ProtocolIEID{Value: 121}, Criticality: n.Criticality{Value: 0}, Value: n.InitialUEMessageIEsValue{Present: n.InitialUEMessageIEsPresentUserLocationInformation, UserLocationInformation: loc}},
			{Id: n.ProtocolIEID{Value: 90}, Criticality: n.Criticality{Value: 1}, Value: n.InitialUEMessageIEsValue{Present: n.InitialUEMessageIEsPresentRRCEstablishmentCause, RRCEstablishmentCause: &n.RRCEstablishmentCause{Value: 3}}},
		}
	} else if ul {
		m.ProcedureCode.Value = 46
		m.Value.Present = n.InitiatingMessagePresentUplinkNASTransport
		m.Value.UplinkNASTransport = &n.UplinkNASTransport{}
		m.Value.UplinkNASTransport.ProtocolIEs.List = []n.UplinkNASTransportIEs{
			{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 0}, Value: n.UplinkNASTransportIEsValue{Present: n.UplinkNASTransportIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
			{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 0}, Value: n.UplinkNASTransportIEsValue{Present: n.UplinkNASTransportIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
			{Id: n.ProtocolIEID{Value: 38}, Criticality: n.Criticality{Value: 0}, Value: n.UplinkNASTransportIEsValue{Present: n.UplinkNASTransportIEsPresentNASPDU, NASPDU: &n.NASPDU{Value: b}}},
			{Id: n.ProtocolIEID{Value: 121}, Criticality: n.Criticality{Value: 1}, Value: n.UplinkNASTransportIEsValue{Present: n.UplinkNASTransportIEsPresentUserLocationInformation, UserLocationInformation: loc}},
		}
	} else {
		m.ProcedureCode.Value = 4
		m.Value.Present = n.InitiatingMessagePresentDownlinkNASTransport
		m.Value.DownlinkNASTransport = &n.DownlinkNASTransport{}
		m.Value.DownlinkNASTransport.ProtocolIEs.List = []n.DownlinkNASTransportIEs{
			{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 0}, Value: n.DownlinkNASTransportIEsValue{Present: n.DownlinkNASTransportIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
			{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 0}, Value: n.DownlinkNASTransportIEsValue{Present: n.DownlinkNASTransportIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
			{Id: n.ProtocolIEID{Value: 38}, Criticality: n.Criticality{Value: 0}, Value: n.DownlinkNASTransportIEsValue{Present: n.DownlinkNASTransportIEsPresentNASPDU, NASPDU: &n.NASPDU{Value: b}}},
		}
	}
	out, e := ngap.Encoder(p)
	must(t, e)
	return out
}
func extract(t *testing.T, wire []byte) []byte {
	t.Helper()
	p, e := ngap.Decoder(wire)
	must(t, e)
	m := p.InitiatingMessage
	check(t, m != nil, "expected initiating NGAP")
	if m.Value.DownlinkNASTransport != nil {
		for _, ie := range m.Value.DownlinkNASTransport.ProtocolIEs.List {
			if ie.Value.NASPDU != nil {
				return ie.Value.NASPDU.Value
			}
		}
	}
	if m.Value.UplinkNASTransport != nil {
		for _, ie := range m.Value.UplinkNASTransport.ProtocolIEs.List {
			if ie.Value.NASPDU != nil {
				return ie.Value.NASPDU.Value
			}
		}
	}
	t.Fatal("missing NAS")
	return nil
}
func kdf(key []byte, fc byte, params ...[]byte) []byte {
	h := hmac.New(sha256.New, key)
	h.Write([]byte{fc})
	for _, p := range params {
		h.Write(p)
		var n [2]byte
		binary.BigEndian.PutUint16(n[:], uint16(len(p)))
		h.Write(n[:])
	}
	return h.Sum(nil)
}

type securityPeer struct {
	enc, integ [16]byte
	ul, dl     uint32
	lastSQN    uint64
}

func labkey(id uint32) []byte {
	b := []byte("RANterra public synthetic USIM v1")
	b = binary.BigEndian.AppendUint32(b, id)
	h := sha256.Sum256(b)
	return h[:16]
}
func (s *securityPeer) derive(t *testing.T, id uint32, rand, autn []byte) []byte {
	t.Helper()
	key := labkey(id)
	opc, e := milenage.GenerateOPc(key, bytes.Repeat([]byte{0x42}, 16))
	must(t, e)
	sqn, _, ik, ck, res, e := milenage.GenerateKeysWithAUTN(opc, key, rand, autn)
	must(t, e)
	n := binary.BigEndian.Uint64(append([]byte{0, 0}, sqn...))
	check(t, n > s.lastSQN, "stale AUTN")
	check(t, autn[6]&0x80 != 0, "missing separation bit")
	s.lastSQN = n
	sn := []byte("5G:mnc001.mcc001.3gppnetwork.org")
	base := append(bytes.Clone(ck), ik...)
	ausf := kdf(base, 0x6a, sn, autn[:6])
	seaf := kdf(ausf, 0x6c, sn)
	amf := kdf(seaf, 0x6d, []byte(fmt.Sprintf("00101%010d", id)), []byte{0, 0})
	copy(s.enc[:], kdf(amf, 0x69, []byte{1}, []byte{2})[16:])
	copy(s.integ[:], kdf(amf, 0x69, []byte{2}, []byte{2})[16:])
	return kdf(base, 0x6b, sn, rand, res)[16:]
}
func (s *securityPeer) seal(t *testing.T, b []byte, sht byte, ul bool) []byte {
	t.Helper()
	count := &s.dl
	dir := byte(1)
	if ul {
		count = &s.ul
		dir = 0
	}
	payload := bytes.Clone(b)
	if sht == 2 || sht == 4 {
		must(t, security.NASEncrypt(2, s.enc, *count, 1, dir, payload))
	}
	mac, e := security.NASMacCalculate(2, s.integ, *count, 1, dir, append([]byte{byte(*count)}, payload...))
	must(t, e)
	out := append([]byte{0x7e, sht}, mac...)
	out = append(out, byte(*count))
	out = append(out, payload...)
	*count++
	return out
}
func (s *securityPeer) open(t *testing.T, b []byte, sht byte, ul bool) []byte {
	t.Helper()
	count := &s.dl
	dir := byte(1)
	if ul {
		count = &s.ul
		dir = 0
	}
	check(t, len(b) >= 10 && b[0] == 0x7e && b[1] == sht && b[6] == byte(*count), "NAS security header/count")
	mac, e := security.NASMacCalculate(2, s.integ, *count, 1, dir, b[6:])
	must(t, e)
	check(t, hmac.Equal(mac, b[2:6]), "independent NAS MAC mismatch")
	out := bytes.Clone(b[7:])
	if sht == 2 || sht == 4 {
		must(t, security.NASEncrypt(2, s.enc, *count, 1, dir, out))
	}
	*count++
	plain(t, out)
	return out
}
func authResponse(t *testing.T, s *securityPeer, id uint32, wire []byte) []byte {
	t.Helper()
	a := plain(t, wire).AuthenticationRequest
	check(t, a != nil && bytes.Equal(a.ABBA.Buffer, []byte{0, 0}), "auth request/ABBA")
	return canonical(t, append(unhex("7e00572d10"), s.derive(t, id, a.AuthenticationParameterRAND.Octet[:], a.AuthenticationParameterAUTN.Octet[:])...))
}
func smComplete(t *testing.T, id uint32) []byte {
	req := registration(id)
	b := binary.BigEndian.AppendUint16(unhex("7e005e71"), uint16(len(req)))
	return canonical(t, append(b, req...))
}
func TestIndependentUEAgainstRustAMF(t *testing.T) {
	h := endpoint(t)
	ids := []uint32{101, 102, 103}
	peers := map[uint32]*securityPeer{}
	challenges := map[uint32][]byte{}
	// Interleave each stage across independent subscribers, not serial whole registrations.
	for _, id := range ids {
		h.ok("new", id, nil, false)
		peers[id] = &securityPeer{}
		challenges[id] = extract(t, h.ok("amf", id, envelope(t, id, canonical(t, registration(id)), true, true), false).Bytes)
	}
	for _, id := range ids {
		s := peers[id]
		res := authResponse(t, s, id, challenges[id])
		smc := extract(t, h.ok("amf", id, envelope(t, id, res, false, true), false).Bytes)
		p := s.open(t, smc, 3, false)
		check(t, bytes.Equal(p, unhex("7e005d220002a020360102")), "algorithms/capabilities")
	}
	for _, id := range ids {
		s := peers[id]
		wire := s.seal(t, smComplete(t, id), 4, true)
		bad := bytes.Clone(wire)
		bad[2] ^= 1
		check(t, h.call("amf", id, envelope(t, id, bad, false, true), false).Error != "", "tampered complete accepted")
		accept := extract(t, h.ok("amf", id, envelope(t, id, wire, false, true), false).Bytes)
		check(t, plain(t, s.open(t, accept, 2, false)).RegistrationAccept != nil, "missing accept")
		done := h.ok("amf", id, envelope(t, id, s.seal(t, unhex("7e0043"), 2, true), false, true), false)
		check(t, done.Registered, "AMF did not verify registration complete")
	}
	for _, id := range ids {
		s := peers[id]
		for i := 0; i < 260; i++ { // COUNT rollover plus replay/tamper/direction rejection without poisoning valid receive.
			payload := canonical(t, unhex("7e00670100082e0101c1ffff91a11201812201012504036c6162"))
			wire := s.seal(t, payload, 2, true)
			bad := bytes.Clone(wire)
			bad[2] ^= 1
			check(t, h.call("receive", id, bad, true).Error != "", "bad MAC accepted")
			if i == 0 {
				check(t, h.call("receive", ids[(int(id)-100)%len(ids)], wire, true).Error != "", "cross-subscriber accepted")
			}
			got := h.ok("receive", id, wire, true).Bytes
			check(t, bytes.Equal(got, payload), "UL plaintext mismatch")
			check(t, h.call("receive", id, wire, true).Error != "", "replay accepted")
			dl := canonical(t, unhex("7e00680100052e0101d3241201"))
			got = h.ok("protect", id, dl, false).Bytes
			check(t, bytes.Equal(s.open(t, got, 2, false), dl), "DL plaintext mismatch")
		}
	}
}
func TestRustUERejectsPlaintextSecurityMode(t *testing.T) {
	h := endpoint(t)
	id := uint32(111)
	h.ok("new", id, nil, false)
	key := labkey(id)
	opc, e := milenage.GenerateOPc(key, bytes.Repeat([]byte{0x42}, 16))
	must(t, e)
	rand := unhex("23553cbe9637a89d218ae64dae47bf35")
	_, _, _, autn, e := milenage.GenerateAKAParameters(opc, key, rand, []byte{0, 0, 0, 0, 0, 1}, []byte{0x80, 0})
	must(t, e)
	challenge := append(unhex("7e00560002000021"), rand...)
	challenge = append(challenge, 0x20, 16)
	challenge = append(challenge, autn...)
	response := extract(t, h.ok("ue", id, envelope(t, id, canonical(t, challenge), false, false), false).Bytes)
	s := &securityPeer{}
	expected := append(unhex("7e00572d10"), s.derive(t, id, rand, autn)...)
	check(t, bytes.Equal(response, expected), "independent RES* mismatch")
	smc := unhex("7e005d220002a020360102")
	check(t, h.call("ue", id, envelope(t, id, smc, false, false), false).Error != "", "UNPROTECTED security mode command accepted")
	wrongDirection := *s
	check(t, h.call("ue", id, envelope(t, id, wrongDirection.seal(t, smc, 3, true), false, false), false).Error != "", "wrong-direction SMC MAC accepted")
	complete := extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, smc, 3, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 4, true), smComplete(t, id)), "SM complete replay mismatch")
	// Losing Complete must be recoverable with a freshly protected SMC, without resetting COUNT.
	complete = extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, smc, 3, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 4, true), smComplete(t, id)), "retransmitted SMC response mismatch")
	accept := canonical(t, unhex("7e00420101"))
	complete = extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, accept, 2, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 2, true), unhex("7e0043")), "UE complete missing")
	complete = extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, accept, 2, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 2, true), unhex("7e0043")), "retransmitted Accept response mismatch")
}

func registerExternalUE(t *testing.T, h *peerHarness, id uint32) *securityPeer {
	h.ok("new", id, nil, false)
	s := &securityPeer{}
	challenge := extract(t, h.ok("amf", id, envelope(t, id, canonical(t, registration(id)), true, true), false).Bytes)
	smc := extract(t, h.ok("amf", id, envelope(t, id, authResponse(t, s, id, challenge), false, true), false).Bytes)
	s.open(t, smc, 3, false)
	accept := extract(t, h.ok("amf", id, envelope(t, id, s.seal(t, smComplete(t, id), 4, true), false, true), false).Bytes)
	s.open(t, accept, 2, false)
	check(t, h.ok("amf", id, envelope(t, id, s.seal(t, unhex("7e0043"), 2, true), false, true), false).Registered, "registration failed")
	return s
}
func checksum(b []byte) uint16 {
	var sum uint32
	for i := 0; i < len(b); i += 2 {
		sum += uint32(b[i]) << 8
		if i+1 < len(b) {
			sum += uint32(b[i+1])
		}
	}
	for sum>>16 != 0 {
		sum = (sum & 65535) + (sum >> 16)
	}
	return ^uint16(sum)
}
func TestIndependentSessionAndUserPlane(t *testing.T) {
	h := endpoint(t)
	id := uint32(121)
	s := registerExternalUE(t, h, id)
	req := canonical(t, unhex("7e00670100082e0101c1ffff91a11201812201012504036c6162"))
	wire := h.ok("setup", id, s.seal(t, req, 2, true), false).Bytes
	p, e := ngap.Decoder(wire)
	must(t, e)
	setup := p.InitiatingMessage.Value.PDUSessionResourceSetupRequest
	check(t, setup != nil, "missing N2 setup")
	found := false
	for _, ie := range setup.ProtocolIEs.List {
		if ie.Value.PDUSessionResourceSetupListSUReq == nil {
			continue
		}
		items := ie.Value.PDUSessionResourceSetupListSUReq.List
		check(t, len(items) == 1, "session count")
		item := items[0]
		check(t, item.PDUSessionID.Value == 1 && bytes.Equal(item.SNSSAI.SST.Value, []byte{1}), "session/slice mismatch")
		dl := plain(t, s.open(t, item.PDUSessionNASPDU.Value, 2, false)).DLNASTransport
		check(t, dl != nil, "missing DL N1")
		sm := plain(t, dl.PayloadContainer.Buffer).PDUSessionEstablishmentAccept
		check(t, sm != nil, "missing SM accept")
		ip := sm.PDUAddress.GetPDUAddressInformation()
		check(t, bytes.Equal(ip[:4], []byte{10, 45, 0, 1}), "IPv4 allocation mismatch")
		var tr n.PDUSessionResourceSetupRequestTransfer
		must(t, aper.UnmarshalWithParams(item.PDUSessionResourceSetupRequestTransfer, &tr, "valueExt"))
		seen := map[int64]bool{}
		for _, v := range tr.ProtocolIEs.List {
			seen[v.Id.Value] = true
			if v.Value.ULNGUUPTNLInformation != nil {
				check(t, bytes.Equal(v.Value.ULNGUUPTNLInformation.GTPTunnel.GTPTEID.Value, []byte{0, 0, 0, 2}), "uplink TEID")
			}
		}
		for _, key := range []int64{130, 134, 136, 139} {
			check(t, seen[key], "required N2 IE absent")
		}
		found = true
	}
	check(t, found, "session list missing")
	tr := n.PDUSessionResourceSetupResponseTransfer{DLQosFlowPerTNLInformation: n.QosFlowPerTNLInformation{UPTransportLayerInformation: n.UPTransportLayerInformation{Present: n.UPTransportLayerInformationPresentGTPTunnel, GTPTunnel: &n.GTPTunnel{TransportLayerAddress: n.TransportLayerAddress{Value: aper.BitString{Bytes: []byte{10, 0, 0, 1}, BitLength: 32}}, GTPTEID: n.GTPTEID{Value: []byte{0, 0, 0, 3}}}}, AssociatedQosFlowList: n.AssociatedQosFlowList{List: []n.AssociatedQosFlowItem{{QosFlowIdentifier: n.QosFlowIdentifier{Value: 1}}}}}}
	b, e := aper.MarshalWithParams(tr, "valueExt")
	must(t, e)
	response := n.NGAPPDU{Present: n.NGAPPDUPresentSuccessfulOutcome, SuccessfulOutcome: &n.SuccessfulOutcome{ProcedureCode: n.ProcedureCode{Value: 29}, Criticality: n.Criticality{Value: 0}, Value: n.SuccessfulOutcomeValue{Present: n.SuccessfulOutcomePresentPDUSessionResourceSetupResponse, PDUSessionResourceSetupResponse: &n.PDUSessionResourceSetupResponse{ProtocolIEs: n.ProtocolIEContainerPDUSessionResourceSetupResponseIEs{List: []n.PDUSessionResourceSetupResponseIEs{
		{Id: n.ProtocolIEID{Value: 75}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentPDUSessionResourceSetupListSURes, PDUSessionResourceSetupListSURes: &n.PDUSessionResourceSetupListSURes{List: []n.PDUSessionResourceSetupItemSURes{{PDUSessionID: n.PDUSessionID{Value: 1}, PDUSessionResourceSetupResponseTransfer: b}}}}},
		{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
	}}}}}}
	b, e = ngap.Encoder(response)
	must(t, e)
	check(t, h.call("setup_response", id, b, false).Error != "", "out-of-order N2 IEs accepted")
	// TS 38.413 9.4.1 requires object-set order: AMF ID, RAN ID, success list.
	list := response.SuccessfulOutcome.Value.PDUSessionResourceSetupResponse.ProtocolIEs.List
	list[0], list[2] = list[2], list[0]
	b, e = ngap.Encoder(response)
	must(t, e)
	h.ok("setup_response", id, b, false)
	packet := unhex("4500002400000000400100000a2d00010a2dff01080000000001000152414e7465727261")
	binary.BigEndian.PutUint16(packet[10:12], checksum(packet[:20]))
	binary.BigEndian.PutUint16(packet[22:24], checksum(packet[20:]))
	gtp := append(unhex("30ff002400000002"), packet...)
	got := h.ok("echo", id, gtp, false).Bytes
	check(t, bytes.Equal(got[:8], unhex("30ff002400000003")), "downlink GTP tunnel")
	reply := got[8:]
	check(t, checksum(reply[:20]) == 0 && checksum(reply[20:]) == 0, "reply checksums")
	check(t, bytes.Equal(reply[12:16], []byte{10, 45, 255, 1}) && bytes.Equal(reply[16:20], []byte{10, 45, 0, 1}) && reply[20] == 0, "echo reply route/type")
	bad := bytes.Clone(gtp)
	bad[7] = 9
	check(t, h.call("echo", id, bad, false).Error != "", "unknown TEID accepted")
}
func TestIndependentWrongAuthenticationResponse(t *testing.T) {
	h := endpoint(t)
	id := uint32(131)
	h.ok("new", id, nil, false)
	challenge := extract(t, h.ok("amf", id, envelope(t, id, registration(id), true, true), false).Bytes)
	res := authResponse(t, &securityPeer{}, id, challenge)
	res[len(res)-1] ^= 1
	reject := extract(t, h.ok("amf", id, envelope(t, id, res, false, true), false).Bytes)
	check(t, bytes.Equal(reject, unhex("7e0058")), "incorrect RES* not rejected")
	check(t, h.call("protect", id, unhex("7e0043"), false).Error != "", "failed subscriber has security context")
}

func TestRustUERejectsIndependentInvalidAUTN(t *testing.T) {
	h := endpoint(t)
	id := uint32(141)
	h.ok("new", id, nil, false)
	key := labkey(id)
	opc, e := milenage.GenerateOPc(key, bytes.Repeat([]byte{0x42}, 16))
	must(t, e)
	rand := unhex("23553cbe9637a89d218ae64dae47bf35")
	_, _, _, autn, e := milenage.GenerateAKAParameters(opc, key, rand, []byte{0, 0, 0, 0, 0, 1}, []byte{0x80, 0})
	must(t, e)
	autn[15] ^= 1
	b := append(unhex("7e00560002000021"), rand...)
	b = append(b, 0x20, 16)
	b = append(b, autn...)
	failure := extract(t, h.ok("ue", id, envelope(t, id, canonical(t, b), false, false), false).Bytes)
	check(t, bytes.Equal(failure, unhex("7e005914")), "UE must emit authentication failure cause20")
	check(t, h.call("protect", id, unhex("7e0043"), true).Error != "", "invalid AUTN left usable security context")
}

func setupReply(t *testing.T, id, teid uint32) []byte {
	tr := n.PDUSessionResourceSetupResponseTransfer{DLQosFlowPerTNLInformation: n.QosFlowPerTNLInformation{UPTransportLayerInformation: n.UPTransportLayerInformation{Present: n.UPTransportLayerInformationPresentGTPTunnel, GTPTunnel: &n.GTPTunnel{TransportLayerAddress: n.TransportLayerAddress{Value: aper.BitString{Bytes: []byte{10, 0, 0, 1}, BitLength: 32}}, GTPTEID: n.GTPTEID{Value: binary.BigEndian.AppendUint32(nil, teid)}}}, AssociatedQosFlowList: n.AssociatedQosFlowList{List: []n.AssociatedQosFlowItem{{QosFlowIdentifier: n.QosFlowIdentifier{Value: 1}}}}}}
	b, e := aper.MarshalWithParams(tr, "valueExt")
	must(t, e)
	response := n.NGAPPDU{Present: n.NGAPPDUPresentSuccessfulOutcome, SuccessfulOutcome: &n.SuccessfulOutcome{ProcedureCode: n.ProcedureCode{Value: 29}, Criticality: n.Criticality{Value: 0}, Value: n.SuccessfulOutcomeValue{Present: n.SuccessfulOutcomePresentPDUSessionResourceSetupResponse, PDUSessionResourceSetupResponse: &n.PDUSessionResourceSetupResponse{ProtocolIEs: n.ProtocolIEContainerPDUSessionResourceSetupResponseIEs{List: []n.PDUSessionResourceSetupResponseIEs{
		{Id: n.ProtocolIEID{Value: 75}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentPDUSessionResourceSetupListSURes, PDUSessionResourceSetupListSURes: &n.PDUSessionResourceSetupListSURes{List: []n.PDUSessionResourceSetupItemSURes{{PDUSessionID: n.PDUSessionID{Value: 1}, PDUSessionResourceSetupResponseTransfer: b}}}}},
		{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
	}}}}}}

	list := response.SuccessfulOutcome.Value.PDUSessionResourceSetupResponse.ProtocolIEs.List
	list[0], list[2] = list[2], list[0]
	b, e = ngap.Encoder(response)
	must(t, e)
	return b
}
