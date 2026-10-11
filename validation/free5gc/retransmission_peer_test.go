package main

import (
	"bytes"
	"github.com/free5gc/aper"
	"github.com/free5gc/ngap"
	n "github.com/free5gc/ngap/ngapType"
	"testing"
)

func eventBytes(t *testing.T, r reply) []byte {
	events := r.Raw["events"].([]any)
	check(t, len(events) == 1, "expected one timer event")
	values := events[0].(map[string]any)["bytes"].([]any)
	b := make([]byte, len(values))
	for i, v := range values {
		b[i] = byte(v.(float64))
	}
	return b
}
func TestIndependentRegistrationRetransmissionTimers(t *testing.T) {
	h := endpoint(t)
	id := uint32(231)
	h.ok("new", id, nil, false)
	s := &securityPeer{}
	challenge := extract(t, h.ok("amf", id, envelope(t, id, registration(id), true, true), false).Bytes)
	h.now = 5999
	check(t, len(h.ok("registration_timers", id, nil, false).Raw["events"].([]any)) == 0, "early T3560")
	h.now = 6000
	retry := extract(t, eventBytes(t, h.ok("registration_timers", id, nil, false)))
	check(t, bytes.Equal(challenge, retry), "AKA retry changed vector")
	smc := extract(t, h.ok("amf", id, envelope(t, id, authResponse(t, s, id, retry), false, true), false).Bytes)
	original := s.open(t, smc, 3, false) // response is lost
	h.now = 12000
	retry = extract(t, eventBytes(t, h.ok("registration_timers", id, nil, false)))
	check(t, !bytes.Equal(smc, retry) && bytes.Equal(original, s.open(t, retry, 3, false)), "SMC retry COUNT/MAC invalid")
	accept := extract(t, h.ok("amf", id, envelope(t, id, s.seal(t, smComplete(t, id), 4, true), false, true), false).Bytes)
	original = s.open(t, accept, 2, false) // final Complete is lost
	h.now = 18000
	retry = extract(t, eventBytes(t, h.ok("registration_timers", id, nil, false)))
	check(t, !bytes.Equal(accept, retry) && bytes.Equal(original, s.open(t, retry, 2, false)), "Accept retry COUNT/MAC invalid")
	check(t, h.ok("amf", id, envelope(t, id, s.seal(t, unhex("7e0043"), 2, true), false, true), false).Registered, "not registered")
	h.now = 60000
	check(t, len(h.ok("registration_timers", id, nil, false).Raw["events"].([]any)) == 0, "completed timer not cancelled")
}
func failedSetupReply(t *testing.T, id uint32) []byte {
	tr, e := aper.MarshalWithParams(n.PDUSessionResourceSetupUnsuccessfulTransfer{Cause: n.Cause{Present: n.CausePresentRadioNetwork, RadioNetwork: &n.CauseRadioNetwork{Value: n.CauseRadioNetworkPresentRadioResourcesNotAvailable}}}, "valueExt")
	must(t, e)
	m := n.PDUSessionResourceSetupResponse{ProtocolIEs: n.ProtocolIEContainerPDUSessionResourceSetupResponseIEs{List: []n.PDUSessionResourceSetupResponseIEs{
		{Id: n.ProtocolIEID{Value: 10}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentAMFUENGAPID, AMFUENGAPID: &n.AMFUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: 85}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentRANUENGAPID, RANUENGAPID: &n.RANUENGAPID{Value: int64(id)}}},
		{Id: n.ProtocolIEID{Value: n.ProtocolIEIDPDUSessionResourceFailedToSetupListSURes}, Criticality: n.Criticality{Value: 1}, Value: n.PDUSessionResourceSetupResponseIEsValue{Present: n.PDUSessionResourceSetupResponseIEsPresentPDUSessionResourceFailedToSetupListSURes, PDUSessionResourceFailedToSetupListSURes: &n.PDUSessionResourceFailedToSetupListSURes{List: []n.PDUSessionResourceFailedToSetupItemSURes{{PDUSessionID: n.PDUSessionID{Value: 1}, PDUSessionResourceSetupUnsuccessfulTransfer: tr}}}}},
	}}}
	p := n.NGAPPDU{Present: n.NGAPPDUPresentSuccessfulOutcome, SuccessfulOutcome: &n.SuccessfulOutcome{ProcedureCode: n.ProcedureCode{Value: 29}, Criticality: n.Criticality{Value: 0}, Value: n.SuccessfulOutcomeValue{Present: n.SuccessfulOutcomePresentPDUSessionResourceSetupResponse, PDUSessionResourceSetupResponse: &m}}}
	b, e := ngap.Encoder(p)
	must(t, e)
	return b
}
func TestIndependentFailedN2ResourceCleanup(t *testing.T) {
	h := researchHarness(t)
	id := uint32(232)
	s := registerExternalUE(t, h, id)
	ip, ul := openResearch(t, h, s, id)
	check(t, h.call("setup_failed", id, failedSetupReply(t, id+1), false).Error != "", "wrong failed UE accepted")
	wire := h.ok("setup_failed", id, failedSetupReply(t, id), false).Bytes
	sm := plain(t, s.open(t, wire, 2, false)).DLNASTransport.PayloadContainer.Buffer
	check(t, bytes.Equal(sm, unhex("2e0100d31a")), "network release PTI/cause incorrect")
	check(t, h.call("echo", id, packet(ip, ul), false).Error != "", "failed tunnel forwards")
	h.ok("release_complete", id, s.seal(t, n1(t, unhex("2e0100d4"), false), 2, true), false)
	check(t, h.call("setup_response", id, setupReply(t, id, 700), false).Error != "", "late success resurrected failed session")
	openResearch(t, h, s, id)
}
