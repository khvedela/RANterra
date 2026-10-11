package main

import (
	"bytes"
	"encoding/binary"
	"github.com/free5gc/aper"
	"github.com/free5gc/ngap"
	n "github.com/free5gc/ngap/ngapType"
	"testing"
)

func n1(t *testing.T, sm []byte, service bool) []byte {
	b := binary.BigEndian.AppendUint16(unhex("7e006701"), uint16(len(sm)))
	b = append(b, sm...)
	b = append(b, 0x12, 1)
	if service {
		b = append(b, unhex("812204020102032509087265736561726368")...)
	} // research / SST2 SD010203
	return canonical(t, b)
}
func releaseReply(t *testing.T, id uint32) []byte {
	tr, e := aper.MarshalWithParams(n.PDUSessionResourceReleaseResponseTransfer{}, "valueExt")
	must(t, e)
	m := n.PDUSessionResourceReleaseResponse{ProtocolIEs: n.ProtocolIEContainerPDUSessionResourceReleaseResponseIEs{List: []n.PDUSessionResourceReleaseResponseIEs{
		{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceReleaseResponseIEsValue{Present: n.PDUSessionResourceReleaseResponseIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceReleaseResponseIEsValue{Present: n.PDUSessionResourceReleaseResponseIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: 70}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceReleaseResponseIEsValue{Present: n.PDUSessionResourceReleaseResponseIEsPresentPDUSessionResourceReleasedListRelRes, PDUSessionResourceReleasedListRelRes: &n.PDUSessionResourceReleasedListRelRes{List: []n.PDUSessionResourceReleasedItemRelRes{{PDUSessionID: n.PDUSessionID{Value: 1}, PDUSessionResourceReleaseResponseTransfer: tr}}}}},
	}}}
	p := n.NGAPPDU{Present: n.NGAPPDUPresentSuccessfulOutcome, SuccessfulOutcome: &n.SuccessfulOutcome{ProcedureCode: n.ProcedureCode{Value: 28}, Criticality: n.Criticality{Value: 0}, Value: n.SuccessfulOutcomeValue{Present: n.SuccessfulOutcomePresentPDUSessionResourceReleaseResponse, PDUSessionResourceReleaseResponse: &m}}}
	b, e := ngap.Encoder(p)
	must(t, e)
	return b
}
func packet(ip []byte, teid uint32) []byte {
	b := unhex("4500002400000000400100000a2d00010a2dff01080000000001000152414e7465727261")
	copy(b[12:16], ip)
	binary.BigEndian.PutUint16(b[10:12], checksum(b[:20]))
	binary.BigEndian.PutUint16(b[22:24], checksum(b[20:]))
	return append(binary.BigEndian.AppendUint32(unhex("30ff0024"), teid), b...)
}
func inspectSetup(t *testing.T, s *securityPeer, wire []byte) ([]byte, uint32) {
	p, e := ngap.Decoder(wire)
	must(t, e)
	m := p.InitiatingMessage.Value.PDUSessionResourceSetupRequest
	check(t, m != nil, "setup missing")
	for _, ie := range m.ProtocolIEs.List {
		if ie.Value.PDUSessionResourceSetupListSUReq == nil {
			continue
		}
		items := ie.Value.PDUSessionResourceSetupListSUReq.List
		check(t, len(items) == 1, "session count")
		v := items[0]
		check(t, bytes.Equal(v.SNSSAI.SST.Value, []byte{2}) && v.SNSSAI.SD != nil && bytes.Equal(v.SNSSAI.SD.Value, []byte{1, 2, 3}), "configured N2 slice missing")
		dl := plain(t, s.open(t, v.PDUSessionNASPDU.Value, 2, false)).DLNASTransport
		sm := plain(t, dl.PayloadContainer.Buffer).PDUSessionEstablishmentAccept
		check(t, sm != nil, "accept missing")
		check(t, bytes.Equal(sm.DNN.Buffer, unhex("087265736561726368")), "configured DNN missing")
		ip := sm.PDUAddress.GetPDUAddressInformation()
		var tr n.PDUSessionResourceSetupRequestTransfer
		must(t, aper.UnmarshalWithParams(v.PDUSessionResourceSetupRequestTransfer, &tr, "valueExt"))
		for _, ie := range tr.ProtocolIEs.List {
			if ie.Value.ULNGUUPTNLInformation != nil {
				return bytes.Clone(ip[:4]), binary.BigEndian.Uint32(ie.Value.ULNGUUPTNLInformation.GTPTunnel.GTPTEID.Value)
			}
		}
	}
	t.Fatal("missing allocation")
	return nil, 0
}
func researchHarness(t *testing.T) *peerHarness {
	h := endpoint(t)
	r := h.command(map[string]any{"op": "configure", "id": 1, "policies": []any{map[string]any{"request": map[string]any{"dnn": "research", "snssai": map[string]any{"sst": 2, "sd": []int{1, 2, 3}}}, "max_sessions": 1}}})
	check(t, r.Error == "", r.Error)
	return h
}
func openResearch(t *testing.T, h *peerHarness, s *securityPeer, id uint32) ([]byte, uint32) {
	return inspectSetup(t, s, h.ok("setup", id, s.seal(t, n1(t, unhex("2e0101c1ffff91a1"), true), 2, true), false).Bytes)
}
func rejectCapacity(t *testing.T, h *peerHarness, s *securityPeer, id uint32) {
	wire := h.ok("setup", id, s.seal(t, n1(t, unhex("2e0101c1ffff91a1"), true), 2, true), false).Bytes
	dl := plain(t, s.open(t, wire, 2, false)).DLNASTransport
	check(t, bytes.Equal(dl.PayloadContainer.Buffer, unhex("2e0101c31a370165")), "capacity cause26/T3396 mismatch")
}
func releaseStart(t *testing.T, h *peerHarness, s *securityPeer, id uint32) {
	r := h.ok("release", id, s.seal(t, n1(t, unhex("2e0101d1"), false), 2, true), false)
	dl := plain(t, s.open(t, r.Bytes, 2, false)).DLNASTransport
	check(t, bytes.Equal(dl.PayloadContainer.Buffer, unhex("2e0101d324")), "release command mismatch")
	a := r.Raw["n2"].([]any)
	b := make([]byte, len(a))
	for i, v := range a {
		b[i] = byte(v.(float64))
	}
	p, e := ngap.Decoder(b)
	must(t, e)
	check(t, p.InitiatingMessage.Value.PDUSessionResourceReleaseCommand != nil, "N2 release command missing")
}
func TestIndependentSMFLifecyclePoliciesAndCleanup(t *testing.T) {
	h := researchHarness(t)
	a, b, c := uint32(201), uint32(202), uint32(203)
	sa, sb, sc := registerExternalUE(t, h, a), registerExternalUE(t, h, b), registerExternalUE(t, h, c)
	// A different DNN cannot allocate, even with the configured slice.
	bad := n1(t, unhex("2e0101c1ffff91a1"), true)
	bad[len(bad)-1] = 'x'
	wire := h.ok("setup", c, sc.seal(t, bad, 2, true), false).Bytes
	dl := plain(t, sc.open(t, wire, 2, false)).DLNASTransport
	check(t, dl.Cause5GMM != nil && dl.Cause5GMM.GetCauseValue() == 91, "unforwarded N1 cause91 absent")
	ip, ul := openResearch(t, h, sa, a)
	check(t, bytes.Equal(ip, []byte{10, 45, 0, 1}), "rejection consumed slot")
	gtp := packet(ip, ul)
	retryIP, retryUL := openResearch(t, h, sa, a)
	check(t, bytes.Equal(retryIP, ip) && retryUL == ul, "duplicate pending request allocated again")
	check(t, h.call("echo", a, gtp, false).Error != "", "forwarded before N2 acknowledgement")
	rejectCapacity(t, h, sb, b) // Pending setup reserves capacity, not only active sessions.
	check(t, h.call("setup_response", a, setupReply(t, b, 700), false).Error != "", "wrong N2 context accepted")
	h.ok("setup_response", a, setupReply(t, a, 700), false)
	got := h.ok("echo", a, gtp, false).Bytes
	check(t, binary.BigEndian.Uint32(got[4:8]) == 700, "gNodeB selected TEID ignored")
	check(t, h.call("echo", b, gtp, false).Error != "", "cross UE forwarding")
	releaseStart(t, h, sa, a)
	check(t, h.call("echo", a, gtp, false).Error != "", "forwarding during release")
	check(t, h.call("release_response", a, releaseReply(t, b), false).Error != "", "wrong N2 release context accepted")
	h.ok("release_response", a, releaseReply(t, a), false)
	h.now = 10000
	rejectCapacity(t, h, sb, b) // N2 alone does not release the reservation.
	h.ok("release_complete", a, sa.seal(t, n1(t, unhex("2e0101d4"), false), 2, true), false)
	check(t, h.call("setup", b, sb.seal(t, n1(t, unhex("2e0101c1ffff91a1"), true), 2, true), false).Error != "", "backoff bypassed")
	h.now = 20000
	ip2, ul2 := openResearch(t, h, sb, b)
	check(t, !bytes.Equal(ip, ip2) && ul != ul2, "allocation was recycled")
	h.ok("setup_response", b, setupReply(t, b, 900), false)
	check(t, h.call("echo", b, gtp, false).Error != "", "stale tunnel accepted")
	h.ok("echo", b, packet(ip2, ul2), false)
	releaseStart(t, h, sb, b)
	h.ok("release_complete", b, sb.seal(t, n1(t, unhex("2e0101d4"), false), 2, true), false)
	h.ok("release_response", b, releaseReply(t, b), false) // Reverse completion order works too.
	openResearch(t, h, sa, a)
}
func TestIndependentSMFT3592AndSetupTimeout(t *testing.T) {
	h := researchHarness(t)
	id := uint32(211)
	s := registerExternalUE(t, h, id)
	ip, ul := openResearch(t, h, s, id)
	h.now = 10000
	r := h.ok("advance", id, nil, false)
	check(t, len(r.Raw["events"].([]any)) == 1, "setup timeout missing")
	check(t, h.call("setup_response", id, setupReply(t, id, 700), false).Error != "", "late N2 setup accepted")
	check(t, h.call("echo", id, packet(ip, ul), false).Error != "", "timed out tunnel forwards")
	openResearch(t, h, s, id)
	h.ok("setup_response", id, setupReply(t, id, 701), false)
	releaseStart(t, h, s, id)
	h.now = 25999
	releaseStart(t, h, s, id) // UE release-request retry does not reset T3592.
	h.ok("release_response", id, releaseReply(t, id), false)
	h.now = 25999
	r = h.ok("advance", id, nil, false)
	check(t, len(r.Raw["events"].([]any)) == 0, "T3592 early")
	h.now = 90000
	r = h.ok("advance", id, nil, false)
	events := r.Raw["events"].([]any)
	check(t, len(events) == 5, "four retries and fifth expiry required")
	for i, e := range events {
		v := e.(map[string]any)
		check(t, v["time_ms"].(float64) == float64(26000+i*16000), "timer drift")
		if i < 4 {
			a := v["bytes"].([]any)
			b := make([]byte, len(a))
			for j, x := range a {
				b[j] = byte(x.(float64))
			}
			dl := plain(t, s.open(t, b, 2, false)).DLNASTransport
			check(t, bytes.Equal(dl.PayloadContainer.Buffer, unhex("2e0101d324")), "retry SM payload changed")
		}
	}
	check(t, h.call("release_complete", id, s.seal(t, n1(t, unhex("2e0101d4"), false), 2, true), false).Error != "", "late complete accepted")
	openResearch(t, h, s, id)
}
