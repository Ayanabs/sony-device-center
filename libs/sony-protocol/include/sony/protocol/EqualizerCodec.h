#pragma once

#include "SemanticTypes.h"
#include "sony/transport/SonyError.h"

namespace sony::protocol {

// ULT WEAR: 57/59 03 <preset> <ULT mode> <count> <bass+10> <five bands+10>.
// The device also answers 56 00 with this extended layout, so the profile may
// explicitly identify that alias. Other legacy devices have no ULT-mode byte.
inline EqualizerState decodeEqualizer(const std::vector<uint8_t>& payload, bool ultBass = false) {
    if (payload.size() < 4 || (payload[0] != 0x57 && payload[0] != 0x59) ||
        (payload[1] != 0x00 && payload[1] != 0x01 && payload[1] != 0x03 && payload[1] != 0x04))
        throw SonyException(SonyErrorCode::InvalidResponse, "Incomplete equalizer response");
    const bool ult = payload[1] == 0x03 || (ultBass && payload[1] == 0x00);
    const size_t countIndex = ult ? 4 : 3;
    if (payload.size() <= countIndex || (ult && payload[3] > 2))
        throw SonyException(SonyErrorCode::InvalidResponse, "Invalid ULT equalizer response");
    const size_t count = payload[countIndex];
    const size_t start = countIndex + 1;
    const bool tenBand = payload[1] == 0x04;
    if ((count != 0 && count != (tenBand ? 10 : 6)) || payload.size() < start + count)
        throw SonyException(SonyErrorCode::InvalidResponse, "Truncated or unexpected equalizer bands");
    EqualizerState state;
    state.preset = payload[2];
    state.ultMode = ult ? payload[3] : -1;
    state.bands.assign(tenBand ? 10 : 5, 0);
    if (count) {
        if (!tenBand) state.clearBass = static_cast<int>(payload[start]) - 10;
        for (size_t i = 0; i < state.bands.size(); ++i)
            state.bands[i] = static_cast<int>(payload[start + i + (tenBand ? 0 : 1)]) - (tenBand ? 0 : 10);
    }
    return state;
}

} // namespace sony::protocol
