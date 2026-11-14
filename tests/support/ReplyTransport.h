#pragma once
#include "sony/transport/FakeTransport.h"
#include "sony/protocol/FrameCodec.h"
#include <atomic>
// Alternating sequence numbers and subtype-specific replies exercise real
// request matching rather than bypassing the session machinery.
class ReplyTransport : public sony::transport::FakeTransport {
public:
    std::atomic<int> ultMode{0};
    std::atomic<bool> pauseWhenRemoved{true};
    std::atomic<bool> rejectPauseChanges{false};
    std::atomic<bool> peripheralSupported{false};
    std::atomic<bool> malformedPeripherals{false};
    std::vector<std::string> attempts;
    std::string failAddress;
    void connect(const sony::transport::DeviceAddress& address) override {
        attempts.push_back(address.str());
        if (address.str() == failAddress) throw sony::SonyException(sony::SonyErrorCode::TransportFailure, "Device is off");
        seq = 0; clearIncoming(); FakeTransport::connect(address);
    }
    size_t send(std::span<const std::byte> data) override {
        const auto count = FakeTransport::send(data);
        std::vector<uint8_t> bytes;
        for (auto b : data) bytes.push_back(static_cast<uint8_t>(b));
        auto frame = sony::protocol::FrameCodec::decode(bytes);
        using namespace sony::protocol;
        if (frame.type == DataType::DataMdrNo2) {
            queueIncoming(FrameCodec::encode({DataType::Ack, static_cast<uint8_t>(1 - (frame.sequence & 1)), {}}));
            if (frame.payload == std::vector<uint8_t>{6,0})
                notify(peripheralSupported ? std::vector<uint8_t>{7,0,1,0x32,2}
                                           : std::vector<uint8_t>{7,0,0}, DataType::DataMdrNo2);
            if (frame.payload == std::vector<uint8_t>{0x36,2})
                notify(malformedPeripherals ? std::vector<uint8_t>{0x37,2,2} : peripheralList(), DataType::DataMdrNo2);
            return count;
        }
        if (frame.type != DataType::DataMdr) return count;
        queueIncoming(FrameCodec::encode({DataType::Ack, static_cast<uint8_t>(1 - (frame.sequence & 1)), {}}));
        const auto& p = frame.payload;
        if (p.empty()) return count;
        std::vector<uint8_t> reply;
        if (p[0] == 0) reply = {1,0};
        if (p[0] == 0x22) {
            if (p[1] == 9) reply = {0x23,9,81,0,79,0};
            else reply = {0x23,p[1],85,0};
        }
        if (p[0] == 0x66) reply = {0x67,p[1],1,1,0,0,0};
        if (p[0] == 0x56) {
            reply = p[1] == 3
                ? std::vector<uint8_t>{0x57,3,0xa0,static_cast<uint8_t>(ultMode.load()),6,10,11,12,13,14,15}
                : std::vector<uint8_t>{0x57,0,0,6,10,11,12,13,14,15};
        }
        if (p[0] == 0xe6) reply = {0xe7,1,0};
        if (p.size() >= 2 && p[0] == 0xf6 && p[1] == 1)
            reply = {0xf7,1,static_cast<uint8_t>(pauseWhenRemoved.load() ? 0 : 1)};
        if (p.size() == 3 && p[0] == 0xf8 && p[1] == 1 && !rejectPauseChanges.load())
            pauseWhenRemoved = p[2] == 0;
        if (!reply.empty()) notify(reply);
        return count;
    }
    static std::vector<uint8_t> peripheralList(uint8_t opcode = 0x37) {
        std::vector<uint8_t> p{opcode,2,3};
        const auto add = [&](std::string address, uint8_t slot, std::string name) {
            p.insert(p.end(), address.begin(), address.end());
            p.insert(p.end(), {slot,0,0,0,static_cast<uint8_t>(name.size())});
            p.insert(p.end(), name.begin(), name.end());
        };
        add("11:22:33:44:55:66", 2, "Ayana\xe2\x80\x99s iPhone");
        add("AA:BB:CC:DD:EE:FF", 1, "ODIZY");
        add("00:11:22:33:44:55", 0, "Old phone");
        p.push_back(1);
        return p;
    }
    void notify(std::vector<uint8_t> payload, sony::protocol::DataType type = sony::protocol::DataType::DataMdr) {
        queueIncoming(sony::protocol::FrameCodec::encode({type,
            static_cast<uint8_t>(seq++ & 1), std::move(payload)}));
    }
private:
    std::atomic<unsigned> seq{0};
};
