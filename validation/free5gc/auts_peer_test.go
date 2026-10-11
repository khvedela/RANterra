package main

import (
	"bytes"
	"github.com/free5gc/util/milenage"
	"testing"
)

func independentAUTS(t *testing.T, id uint32, rand []byte, sqn uint64) []byte {
	k := labkey(id)
	opc, e := milenage.GenerateOPc(k, bytes.Repeat([]byte{0x42}, 16))
	must(t, e)
	b := []byte{byte(sqn >> 40), byte(sqn >> 32), byte(sqn >> 24), byte(sqn >> 16), byte(sqn >> 8), byte(sqn)}
	auts, e := milenage.GenerateAUTS(opc, k, rand, b)
	must(t, e)
	return canonical(t, append(unhex("7e005915300e"), auts...))
}
func finishAMF(t *testing.T, h *peerHarness, id uint32, s *securityPeer, challenge []byte) {
	smc := extract(t, h.ok("amf", id, envelope(t, id, authResponse(t, s, id, challenge), false, true), false).Bytes)
	s.open(t, smc, 3, false)
	accept := extract(t, h.ok("amf", id, envelope(t, id, s.seal(t, smComplete(t, id), 4, true), false, true), false).Bytes)
	s.open(t, accept, 2, false)
	check(t, h.ok("amf", id, envelope(t, id, s.seal(t, unhex("7e0043"), 2, true), false, true), false).Registered, "registration after AUTS failed")
}
func TestIndependentAUTSResynchronizesRustAMF(t *testing.T) {
	h := endpoint(t)
	id := uint32(221)
	h.ok("new", id, nil, false)
	challenge := extract(t, h.ok("amf", id, envelope(t, id, registration(id), true, true), false).Bytes)
	a := plain(t, challenge).AuthenticationRequest
	failure := independentAUTS(t, id, a.AuthenticationParameterRAND.Octet[:], 100)
	next := extract(t, h.ok("amf", id, envelope(t, id, failure, false, true), false).Bytes)
	b := plain(t, next).AuthenticationRequest
	check(t, !bytes.Equal(a.AuthenticationParameterRAND.Octet[:], b.AuthenticationParameterRAND.Octet[:]), "resync reused RAND")
	s := &securityPeer{lastSQN: 100}
	finishAMF(t, h, id, s, next)
	check(t, s.lastSQN == 101, "UDM SQN recovery mismatch")
}
func TestIndependentAUTSNegativeCases(t *testing.T) {
	for _, kind := range []string{"mac", "rand", "subscriber", "replay", "exhausted", "truncated"} {
		t.Run(kind, func(t *testing.T) {
			h := endpoint(t)
			id := uint32(231)
			h.ok("new", id, nil, false)
			challenge := extract(t, h.ok("amf", id, envelope(t, id, registration(id), true, true), false).Bytes)
			a := plain(t, challenge).AuthenticationRequest
			rand := bytes.Clone(a.AuthenticationParameterRAND.Octet[:])
			subscriber := id
			sqn := uint64(100)
			if kind == "rand" {
				rand[0] ^= 1
			}
			if kind == "subscriber" {
				subscriber++
			}
			if kind == "exhausted" {
				sqn = (1 << 48) - 1
			}
			failure := independentAUTS(t, subscriber, rand, sqn)
			if kind == "mac" {
				failure[len(failure)-1] ^= 1
			}
			if kind == "replay" {
				h.ok("amf", id, envelope(t, id, failure, false, true), false)
			}
			if kind == "truncated" {
				failure = failure[:len(failure)-1]
				check(t, h.call("amf", id, envelope(t, id, failure, false, true), false).Error != "", "truncated AUTS accepted")
				return
			}
			reject := extract(t, h.ok("amf", id, envelope(t, id, failure, false, true), false).Bytes)
			check(t, bytes.Equal(reject, unhex("7e0058")), "invalid AUTS not rejected")
			check(t, h.call("protect", id, unhex("7e0043"), false).Error != "", "invalid AUTS established keys")
		})
	}
}
func TestIndependentNetworkVerifiesRustAUTSAndAuthRetransmission(t *testing.T) {
	h := endpoint(t)
	id := uint32(241)
	r := h.command(map[string]any{"op": "new", "id": id, "sqn": 100})
	check(t, r.Error == "", r.Error)
	k := labkey(id)
	opc, e := milenage.GenerateOPc(k, bytes.Repeat([]byte{0x42}, 16))
	must(t, e)
	rand := unhex("23553cbe9637a89d218ae64dae47bf35")
	challenge := func(sqn byte) []byte {
		_, _, _, autn, e := milenage.GenerateAKAParameters(opc, k, rand, []byte{0, 0, 0, 0, 0, sqn}, []byte{0x80, 0})
		must(t, e)
		b := append(unhex("7e00560002000021"), rand...)
		b = append(b, 0x20, 16)
		b = append(b, autn...)
		return canonical(t, b)
	}
	stale := challenge(1)
	failure := extract(t, h.ok("ue", id, envelope(t, id, stale, false, false), false).Bytes)
	check(t, len(failure) == 20 && bytes.Equal(failure[:6], unhex("7e005915300e")), "missing synchronization failure cause21/AUTS")
	recovered, e := milenage.ValidateAUTS(opc, k, rand, failure[6:])
	must(t, e)
	check(t, bytes.Equal(recovered, []byte{0, 0, 0, 0, 0, 100}), "Rust AUTS SQN mismatch")
	// Repeated stale challenge must not authenticate or update the USIM counter.
	again := extract(t, h.ok("ue", id, envelope(t, id, stale, false, false), false).Bytes)
	check(t, bytes.Equal(failure, again), "unstable AUTS")
	rand[0] ^= 1
	fresh := challenge(101)
	response := extract(t, h.ok("ue", id, envelope(t, id, fresh, false, false), false).Bytes)
	s := &securityPeer{lastSQN: 100}
	expected := authResponse(t, s, id, fresh)
	check(t, bytes.Equal(response, expected), "RES* after resync mismatch")
	again = extract(t, h.ok("ue", id, envelope(t, id, fresh, false, false), false).Bytes)
	check(t, bytes.Equal(response, again), "authentication retransmission lost RES*")
	complete := extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, unhex("7e005d220002a020360102"), 3, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 4, true), smComplete(t, id)), "SMC complete after retry mismatch")
	check(t, h.call("ue", id, envelope(t, id, fresh, false, false), false).Error != "", "challenge replay after security accepted")
	complete = extract(t, h.ok("ue", id, envelope(t, id, s.seal(t, unhex("7e00420101"), 2, false), false, false), false).Bytes)
	check(t, bytes.Equal(s.open(t, complete, 2, true), unhex("7e0043")), "registration complete after AUTS missing")
}

func TestIndependentAUTSBoundedRepeatedResynchronization(t *testing.T) {
	h := endpoint(t)
	id := uint32(251)
	h.ok("new", id, nil, false)
	challenge := extract(t, h.ok("amf", id, envelope(t, id, registration(id), true, true), false).Bytes)
	for attempt := 1; attempt <= 3; attempt++ {
		a := plain(t, challenge).AuthenticationRequest
		failure := independentAUTS(t, id, a.AuthenticationParameterRAND.Octet[:], uint64(attempt*100))
		challenge = extract(t, h.ok("amf", id, envelope(t, id, failure, false, true), false).Bytes)
		if attempt < 3 {
			check(t, plain(t, challenge).AuthenticationRequest != nil, "resynchronization failed early")
		} else {
			check(t, bytes.Equal(challenge, unhex("7e0058")), "unbounded resynchronization loop")
		}
	}
}
