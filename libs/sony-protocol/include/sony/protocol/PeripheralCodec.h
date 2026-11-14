#pragma once

#include "SemanticTypes.h"
#include "sony/transport/SonyError.h"
#include <cctype>
#include <span>
#include <utility>

namespace sony::protocol {

// V2 table-2 PERI RET/NTFY PARAM. Subtype 0 adds a length-prefixed name
// after the address and connection slot; subtype 2 also carries a 3-byte CoD.
// Packet layout corroborated with ULT WEAR 2.1.0 captures. No setters here.
inline std::vector<ConnectedDevice> decodeConnectedDevices(std::span<const uint8_t> payload) {
    const auto invalid = [] {
        throw SonyException(SonyErrorCode::InvalidResponse, "Malformed headset device list");
    };
    if (payload.size() < 4 || (payload[0] != 0x37 && payload[0] != 0x39) ||
        (payload[1] != 0 && payload[1] != 2)) invalid();
    std::vector<ConnectedDevice> devices;
    std::vector<uint8_t> slots;
    size_t pos = 3;
    for (unsigned i = 0; i < payload[2]; ++i) {
        const size_t fixed = payload[1] == 2 ? 22 : 19;
        if (payload.size() - pos < fixed) invalid();
        std::string address(payload.begin() + pos, payload.begin() + pos + 17);
        for (size_t j = 0; j < address.size(); ++j) {
            const auto ch = static_cast<unsigned char>(address[j]);
            if (j % 3 == 2 ? ch != ':' : !std::isxdigit(ch)) invalid();
            address[j] = static_cast<char>(std::toupper(ch));
        }
        pos += 17;
        const auto slot = payload[pos++];
        if (payload[1] == 2) pos += 3;
        const auto length = payload[pos++];
        if (payload.size() - pos < length) invalid();
        std::string name(payload.begin() + pos, payload.begin() + pos + length);
        pos += length;
        if (slot) {
            devices.push_back({std::move(address), std::move(name), false});
            slots.push_back(slot);
        }
    }
    if (payload.size() - pos != 1) invalid();
    const auto playbackSlot = payload[pos];
    for (size_t i = 0; i < devices.size(); ++i)
        devices[i].playbackDevice = playbackSlot != 0 && slots[i] == playbackSlot;
    return devices;
}
} // namespace sony::protocol
