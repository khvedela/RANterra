// Independent codec cross-check. Validation-only; not linked into the application.
package main

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"github.com/free5gc/aper"
	"github.com/free5gc/nas"
	"github.com/free5gc/ngap"
	"github.com/free5gc/ngap/ngapType"
	"io"
	"os"
)

type Frame struct {
	Protocol string `json:"protocol"`
	Name     string `json:"name"`
	Hex      string `json:"hex"`
}

func main() {
	defer func() {
		if failure := recover(); failure != nil {
			fmt.Fprintf(os.Stderr, "Independent validator rejected input: %v\n", failure)
			os.Exit(1)
		}
	}()
	var frames []Frame
	if err := json.NewDecoder(io.LimitReader(os.Stdin, 1_000_000)).Decode(&frames); err != nil {
		panic(err)
	}
	if len(frames) != 30 {
		panic("Expected exactly 30 engine-generated frames")
	}
	for _, f := range frames {
		if len(f.Hex) > 131072 {
			panic("Frame exceeds validation limit")
		}
		wire, err := hex.DecodeString(f.Hex)
		if err != nil {
			panic(err)
		}
		var encoded []byte
		switch f.Protocol {
		case "NAS":
			m := nas.NewMessage()
			data := append([]byte(nil), wire...)
			if err = m.PlainNasDecode(&data); err == nil {
				encoded, err = m.PlainNasEncode()
			}
		case "NGAP":
			m, e := ngap.Decoder(wire)
			err = e
			if err == nil {
				encoded, err = ngap.Encoder(*m)
				if err != nil {
					b, _ := json.Marshal(m)
					fmt.Fprintln(os.Stderr, string(b))
				}
			}
			if err == nil && f.Name == "PDU-resource-setup" {
				message := m.InitiatingMessage.Value.PDUSessionResourceSetupRequest
				found := false
				for _, ie := range message.ProtocolIEs.List {
					if ie.Value.PDUSessionResourceSetupListSUReq == nil {
						continue
					}
					for _, item := range ie.Value.PDUSessionResourceSetupListSUReq.List {
						var transfer ngapType.PDUSessionResourceSetupRequestTransfer
						if e := aper.UnmarshalWithParams(item.PDUSessionResourceSetupRequestTransfer, &transfer, "valueExt"); e != nil {
							panic(e)
						}
						required := map[int64]bool{130: false, 134: false, 136: false, 139: false}
						for _, field := range transfer.ProtocolIEs.List {
							required[field.Id.Value] = true
						}
						for id, present := range required {
							if !present {
								panic(fmt.Sprintf("Missing N2 transfer IE %d", id))
							}
						}
						b, e := aper.MarshalWithParams(transfer, "valueExt")
						if e != nil || !bytes.Equal(b, item.PDUSessionResourceSetupRequestTransfer) {
							panic("N2 request transfer mismatch")
						}
						found = true
					}
				}
				if !found {
					panic("Missing PDU resource setup list")
				}
			}
		default:
			panic("Unknown protocol")
		}
		if err != nil {
			panic(fmt.Sprintf("%s %s: %v", f.Protocol, f.Name, err))
		}
		if !bytes.Equal(wire, encoded) {
			panic(fmt.Sprintf("%s %s: bytes changed: %x != %x", f.Protocol, f.Name, wire, encoded))
		}
	}
	fmt.Printf("free5GC independently decoded/re-encoded %d NAS/NGAP frames exactly\n", len(frames))
}
