#include <catch2/catch_test_macros.hpp>
#include "sony/protocol/ProtocolV2.h"
#include "sony/protocol/FrameCodec.h"
#include "sony/transport/FakeTransport.h"
#include "ReplyingFakeTransport.h"
#include "sony/protocol/PeripheralCodec.h"

using namespace sony;
using namespace sony::protocol;
using namespace sony::transport;
using sony::test::ReplyingFakeTransport;

namespace {
std::vector<uint8_t> deviceList(uint8_t subtype, uint8_t opcode = 0x37) {
    std::vector<uint8_t> p{opcode, subtype, 3};
    const auto add = [&](std::string address, uint8_t slot, std::string name) {
        p.insert(p.end(), address.begin(), address.end());
        p.push_back(slot);
        if (subtype == 2) p.insert(p.end(), {0x7a,2,0x0c});
        p.push_back(static_cast<uint8_t>(name.size()));
        p.insert(p.end(), name.begin(), name.end());
    };
    add("14:C2:13:C1:51:CC", 2, "Ayana\xe2\x80\x99s iPhone");
    add("e0:0a:f6:c3:fa:24", 1, "ODIZY");
    add("00:11:22:33:44:55", 0, "Old phone");
    p.push_back(1);
    return p;
}
}

TEST_CASE("Connected devices: decode both headset list formats and playback slots", "[protocol][v2][connected-devices]") {
    for (uint8_t subtype : {0, 2}) {
        for (uint8_t opcode : {0x37, 0x39}) {
            const auto list = decodeConnectedDevices(deviceList(subtype, opcode));
            REQUIRE(list.size() == 2);
            CHECK(list[0].name == "Ayana\xe2\x80\x99s iPhone");
            CHECK_FALSE(list[0].playbackDevice);
            CHECK(list[1].address == "E0:0A:F6:C3:FA:24");
            CHECK(list[1].playbackDevice);
        }
    }
    CHECK(decodeConnectedDevices(std::vector<uint8_t>{0x37,2,0,0}).empty());
    auto noSource = deviceList(2); noSource.back() = 0;
    const auto devices = decodeConnectedDevices(noSource);
    CHECK_FALSE(devices[0].playbackDevice);
    CHECK_FALSE(devices[1].playbackDevice);
}

TEST_CASE("Connected devices: reject truncated and malformed lists", "[protocol][v2][connected-devices]") {
    for (uint8_t subtype : {0, 2}) {
        const auto p = deviceList(subtype);
        for (size_t length = 0; length < p.size(); ++length)
            REQUIRE_THROWS_AS(decodeConnectedDevices(std::span(p).first(length)), SonyException);
        auto trailing = p; trailing.push_back(0);
        CHECK_THROWS_AS(decodeConnectedDevices(trailing), SonyException);
        auto badAddress = p; badAddress[3] = 'Z';
        CHECK_THROWS_AS(decodeConnectedDevices(badAddress), SonyException);
        auto badCount = p; ++badCount[2];
        CHECK_THROWS_AS(decodeConnectedDevices(badCount), SonyException);
        auto badSubtype = p; badSubtype[1] = 1;
        CHECK_THROWS_AS(decodeConnectedDevices(badSubtype), SonyException);
    }
}

TEST_CASE("Connected devices: inquire advertised table-2 list without writes", "[protocol][v2][connected-devices]") {
    for (uint8_t subtype : {0, 2}) {
        ReplyingFakeTransport fake;
        SonyProtocolSession session(&fake);
        session.connect("11:22:33:44:55:66");
        ProtocolV2 protocol(session);
        fake.queueReply({{DataType::Ack,1,{}}, {DataType::DataMdrNo2,0,
            {7,0,1,static_cast<uint8_t>(subtype == 2 ? 0x32 : 0x30),2}}});
        fake.queueReply({{DataType::Ack,0,{}}, {DataType::DataMdrNo2,1,deviceList(subtype)}});
        const auto devices = protocol.getConnectedDevices();
        REQUIRE(devices.size() == 2);
        CHECK(devices[1].playbackDevice);
        std::vector<std::vector<uint8_t>> requests;
        for (const auto& raw : fake.sentFrames()) {
            const auto frame = FrameCodec::decode(raw);
            if (frame.type == DataType::Ack) continue;
            REQUIRE(frame.type == DataType::DataMdrNo2);
            requests.push_back(frame.payload);
        }
        REQUIRE(requests == std::vector<std::vector<uint8_t>>{{6,0},{0x36,subtype}});
    }
}

TEST_CASE("Connected devices: absent support sends no device-list probe", "[protocol][v2][connected-devices]") {
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");
    ProtocolV2 protocol(session);
    fake.queueReply({{DataType::Ack,1,{}}, {DataType::DataMdrNo2,0,{7,0,1,0x42,0xff}}});
    try { protocol.getConnectedDevices(); FAIL("Expected Unsupported"); }
    catch (const SonyException& ex) { CHECK(ex.code() == SonyErrorCode::Unsupported); }
    unsigned requests = 0;
    for (const auto& raw : fake.sentFrames()) {
        const auto frame = FrameCodec::decode(raw);
        if (frame.type == DataType::Ack) continue;
        CHECK(frame.payload == std::vector<uint8_t>{6,0});
        ++requests;
    }
    CHECK(requests == 1);
}

TEST_CASE("ProtocolV2: uses opcode 0x22 for battery request", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session);
    REQUIRE(v2.generation() == ProtocolGeneration::V2);

    // Reply for single battery: 23 00 80 01 (80%, charging)
    fake.queueReply({
        SonyFrame{ .type = DataType::Ack, .sequence = 0 },
        SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0x23, 0x00, 80, 1} }
    });

    auto battery = v2.getBattery();
    REQUIRE(battery.main.has_value());
    REQUIRE(*battery.main == 80);
    REQUIRE(battery.charging == true);

    // Verify opcode 0x22 WAS sent
    bool found0x22 = false;
    for (const auto& frameBytes : fake.sentFrames()) {
        auto decoded = FrameCodec::decode(frameBytes);
        if (decoded.type == DataType::DataMdr && !decoded.payload.empty() && decoded.payload[0] == 0x22) {
            found0x22 = true;
            break;
        }
    }
    REQUIRE(found0x22);
}

TEST_CASE("ProtocolV2: handles noise control query and command", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session);

    SECTION("getNoiseControl decodes ambient mode")
    {
        // Reply: 67 17 01 <on=1> <ambient=1> <voice=0> <level=12>
        fake.queueReply({
            SonyFrame{ .type = DataType::Ack, .sequence = 0 },
            SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0x67, 0x17, 0x01, 1, 1, 0, 12} }
        });

        auto nc = v2.getNoiseControl();
        REQUIRE(nc.mode == NoiseControlMode::Ambient);
        REQUIRE(nc.ambientLevel == 12);
        REQUIRE(nc.focusOnVoice == false);
    }

    SECTION("setNoiseControl sends V2 layout")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });

        NoiseControlState state{
            .mode = NoiseControlMode::NoiseCancelling,
            .ambientLevel = 0,
            .focusOnVoice = false
        };
        v2.setNoiseControl(state);

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x68, 0x17, 0x01, 1, 0, 0, 1});
    }
}

TEST_CASE("ProtocolV2: handles equalizer queries and settings", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session);

    SECTION("getEqualizer decodes preset and bands")
    {
        // Reply: 57 00 <preset=0x16: Bass Boost> 06 <clearBass=13 (+3)> <10, 11, 12, 13, 14 (0, +1, +2, +3, +4)>
        fake.queueReply({
            SonyFrame{ .type = DataType::Ack, .sequence = 0 },
            SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0x57, 0x00, 0x16, 0x06, 13, 10, 11, 12, 13, 14} }
        });

        auto eq = v2.getEqualizer();
        REQUIRE(eq.preset == 0x16);
        REQUIRE(eq.clearBass == 3);
        REQUIRE(eq.bands == std::vector<int>{0, 1, 2, 3, 4});
    }

    SECTION("setEqualizerPreset sends command")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });
        v2.setEqualizerPreset(0x16);

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x58, 0x00, 0x16, 0x00});
    }

    SECTION("setEqualizerCustom sends clamped bands")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });
        v2.setEqualizerCustom(5, { -2, 0, 3, 7, 10 });

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x58, 0x00, 0xa0, 0x06, 15, 8, 10, 13, 17, 20});
    }
}

TEST_CASE("ProtocolV2: ULT WEAR EQ preserves its separate bass mode", "[protocol][v2][ult]") {
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");
    ProtocolV2 protocol(session, false, true);
    fake.queueReply({
        SonyFrame{DataType::Ack, 1, {}},
        SonyFrame{DataType::DataMdr, 0, {0x57,3,0xa0,2,6,10,11,12,13,14,15}}
    });
    SECTION("query decodes the ULT byte separately from the count") {
        const auto eq = protocol.getEqualizer();
        REQUIRE(eq.ultMode == 2);
        REQUIRE(eq.clearBass == 0);
        REQUIRE(eq.bands == std::vector<int>{1,2,3,4,5});
        REQUIRE(FrameCodec::decode(fake.sentFrames().front()).payload == std::vector<uint8_t>{0x56,3});
    }
    SECTION("custom EQ includes and preserves ULT mode") {
        fake.queueReply({SonyFrame{DataType::Ack, 0, {}}});
        protocol.setEqualizerCustom(5, {2,-3,-4,0,1});
        REQUIRE(FrameCodec::decode(fake.lastSentFrame()).payload == std::vector<uint8_t>{0x58,3,0xa0,2,6,15,12,7,6,10,11});
    }
    SECTION("preset includes and preserves ULT mode") {
        fake.queueReply({SonyFrame{DataType::Ack, 0, {}}});
        protocol.setEqualizerPreset(0x16);
        REQUIRE(FrameCodec::decode(fake.lastSentFrame()).payload == std::vector<uint8_t>{0x58,3,0x16,2,0});
    }
}

// WH-1000XM6 speaks a different, 10-band equalizer layout under inquired
// type 0x04 instead of the legacy 5-band + Clear Bass layout under 0x00 --
// reverse-engineered from a real Sound Connect btsnoop capture (issue #10).
// Frame bytes below are taken directly from that capture.
TEST_CASE("ProtocolV2: handles the 10-band equalizer layout (WH-1000XM6)", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session, /*tenBandEqualizer=*/true);

    SECTION("getEqualizer queries inquired type 0x04 and decodes 10 raw bands")
    {
        // Capture offset 22375: 57 04 a0 0a 07 07 07 07 06 06 07 08 08 08
        fake.queueReply({
            SonyFrame{ .type = DataType::Ack, .sequence = 0 },
            SonyFrame{ .type = DataType::DataMdr, .sequence = 1,
                .payload = {0x57, 0x04, 0xa0, 0x0a, 0x07, 0x07, 0x07, 0x07, 0x06, 0x06, 0x07, 0x08, 0x08, 0x08} }
        });

        auto eq = v2.getEqualizer();
        REQUIRE(fake.sentCount() >= 1);
        REQUIRE(FrameCodec::decode(fake.sentFrames().front()).payload == std::vector<uint8_t>{0x56, 0x04});
        REQUIRE(eq.preset == 0xa0);
        REQUIRE(eq.clearBass == 0);
        REQUIRE(eq.bands == std::vector<int>{7, 7, 7, 7, 6, 6, 7, 8, 8, 8});
    }

    SECTION("setEqualizerPreset uses inquired type 0x04")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });
        v2.setEqualizerPreset(0x30);

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x58, 0x04, 0x30, 0x00});
    }

    SECTION("setEqualizerCustom sends raw, unbiased band values with no Clear Bass slot")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });
        // clearBass is meaningless on this layout and must be ignored.
        v2.setEqualizerCustom(5, {7, 7, 7, 7, 6, 6, 3, 7, 3, 8});

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x58, 0x04, 0xa0, 0x0a, 7, 7, 7, 7, 6, 6, 3, 7, 3, 8});
    }

    SECTION("setEqualizerCustom clamps to the observed 0..12 raw range")
    {
        fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 0 } });
        v2.setEqualizerCustom(0, {-5, 0, 6, 12, 99, -1, 1, 5, 11, 13});

        REQUIRE(fake.sentCount() == 1);
        auto sent = FrameCodec::decode(fake.lastSentFrame());
        REQUIRE(sent.payload == std::vector<uint8_t>{0x58, 0x04, 0xa0, 0x0a, 0, 0, 6, 12, 12, 0, 1, 5, 11, 12});
    }
}

TEST_CASE("ProtocolV2: pause-on-removal uses the inverted V2 flag", "[protocol][v2][pause]") {
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");
    ProtocolV2 protocol(session);
    SECTION("zero means enabled") {
        fake.queueReply({SonyFrame{DataType::Ack,1,{}}, SonyFrame{DataType::DataMdr,0,{0xf7,1,0}}});
        REQUIRE(protocol.getPauseWhenRemoved());
        REQUIRE(FrameCodec::decode(fake.sentFrames().front()).payload == std::vector<uint8_t>{0xf6,1});
    }
    SECTION("one means disabled") {
        fake.queueReply({SonyFrame{DataType::Ack,1,{}}, SonyFrame{DataType::DataMdr,0,{0xf7,1,1}}});
        REQUIRE_FALSE(protocol.getPauseWhenRemoved());
    }
    SECTION("writes use the inverted flag") {
        fake.queueReply({SonyFrame{DataType::Ack,1,{}}});
        protocol.setPauseWhenRemoved(true);
        REQUIRE(FrameCodec::decode(fake.lastSentFrame()).payload == std::vector<uint8_t>{0xf8,1,0});
        fake.queueReply({SonyFrame{DataType::Ack,0,{}}});
        protocol.setPauseWhenRemoved(false);
        REQUIRE(FrameCodec::decode(fake.lastSentFrame()).payload == std::vector<uint8_t>{0xf8,1,1});
    }
    SECTION("unknown flag is rejected") {
        fake.queueReply({SonyFrame{DataType::Ack,1,{}}, SonyFrame{DataType::DataMdr,0,{0xf7,1,0xff}}});
        REQUIRE_THROWS_AS(protocol.getPauseWhenRemoved(), SonyException);
    }
}

TEST_CASE("ProtocolV2: handles DSEE query and control", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session);

    fake.queueReply({
        SonyFrame{ .type = DataType::Ack, .sequence = 0 },
        SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0xe7, 0x01, 0x01} }
    });

    bool dsee = v2.getDsee();
    REQUIRE(dsee == true);

    fake.queueReply({ SonyFrame{ .type = DataType::Ack, .sequence = 1 } });
    v2.setDsee(false);

    auto sent = FrameCodec::decode(fake.lastSentFrame());
    REQUIRE(sent.payload == std::vector<uint8_t>{0xe8, 0x01, 0x00});
}

TEST_CASE("ProtocolV2: handles peripheral feature inquiries", "[protocol][v2]")
{
    ReplyingFakeTransport fake;
    SonyProtocolSession session(&fake);
    session.connect("11:22:33:44:55:66");

    ProtocolV2 v2(session);

    SECTION("firmware version")
    {
        fake.queueReply({
            SonyFrame{ .type = DataType::Ack, .sequence = 0 },
            SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0x05, 0x02, 0x00, '2', '.', '0', '.', '1'} }
        });

        auto fw = v2.getFirmwareVersion();
        REQUIRE(fw == "2.0.1");
    }

    SECTION("codec inquiry")
    {
        fake.queueReply({
            SonyFrame{ .type = DataType::Ack, .sequence = 0 },
            SonyFrame{ .type = DataType::DataMdr, .sequence = 1, .payload = {0x13, 0x02, 0x10} }
        }); // 0x10 = LDAC

        auto codec = v2.getCodec();
        REQUIRE(codec == "LDAC");
    }
}
