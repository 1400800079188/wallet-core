// Copyright © 2017-2023 Trust Wallet.
//
// This file is part of Trust. The full Trust copyright notice, including
// terms governing use, modification, and redistribution, is contained in the
// file LICENSE at the root of the source code distribution tree.

#pragma once

#include <memory>
#include <optional>

#include "../Data.h"
#include "bindgen/WalletCoreRSBindgen.h"

namespace TW::Rust {

inline std::shared_ptr<TWAnyAddress> wrapTWAnyAddress(TWAnyAddress* anyAddress) {
    return std::shared_ptr<TWAnyAddress>(anyAddress, tw_any_address_delete);
}

inline std::shared_ptr<TWPublicKey> wrapTWPublicKey(TWPublicKey* publicKey) {
    return std::shared_ptr<TWPublicKey>(publicKey, tw_public_key_delete);
}

struct TWDataVectorWrapper {
    /// Implicit constructor.
    TWDataVectorWrapper(const std::vector<Data>& vec) {
        ptr = std::shared_ptr<TWDataVector>(tw_data_vector_create(), Rust::tw_data_vector_delete);

        for (const auto& item : vec) {
            auto* itemData = tw_data_create_with_bytes(item.data(), item.size());
            Rust::tw_data_vector_add(ptr.get(), itemData);
            Rust::tw_data_delete(itemData);
        }
    }

    ~TWDataVectorWrapper() = default;

    TWDataVector* get() const {
        return ptr.get();
    }

    std::shared_ptr<TWDataVector> ptr;
};

struct TWDataWrapper {
    /// Implicit constructor.
    TWDataWrapper(const Data& bytes) {
        auto* dataRaw = tw_data_create_with_bytes(bytes.data(), bytes.size());
        ptr = std::shared_ptr<TWData>(dataRaw, tw_data_delete);
    }

    /// Implicit constructor.
    TWDataWrapper(TWData *ptr): ptr(std::shared_ptr<TWData>(ptr, tw_data_delete)) {
    }

    ~TWDataWrapper() = default;

    TWData* get() const {
        return ptr.get();
    }

    Data toDataOrDefault() const {
        if (!ptr) {
            return {};
        }

        auto* bytes = tw_data_bytes(ptr.get());
        Data out(bytes, bytes + tw_data_size(ptr.get()));
        return out;
    }

    std::shared_ptr<TWData> ptr;
};

struct TWStringWrapper {
    /// Implicit constructor.
    TWStringWrapper(const std::string& string) {
        auto* stringRaw = tw_string_create_with_utf8_bytes(string.c_str());
        ptr = std::shared_ptr<TWString>(stringRaw, tw_string_delete);
    }

    /// Implicit constructor.
    TWStringWrapper(TWString *ptr): ptr(std::shared_ptr<TWString>(ptr, tw_string_delete)) {
    }

    ~TWStringWrapper() = default;

    TWString* get() const {
        return ptr.get();
    }

    std::string toStringOrDefault() const {
        if (!ptr) {
            return {};
        }

        auto* bytes = tw_string_utf8_bytes(ptr.get());
        return {bytes};
    }

    std::shared_ptr<TWString> ptr;
};

struct CByteArrayWrapper {
    CByteArrayWrapper() = default;

    /// Implicit constructor.
    CByteArrayWrapper(const CByteArray &rawArray) {
        *this = rawArray;
    }

    CByteArrayWrapper& operator=(CByteArray rawArray) {
        if (rawArray.data == nullptr || rawArray.size == 0) {
            return *this;
        }
        Data result(&rawArray.data[0], &rawArray.data[rawArray.size]);
        free_c_byte_array(&rawArray);
        data = std::move(result);
        return *this;
    }

    Data data;
};

struct CStringWrapper {
    /// Implicit move constructor.
    CStringWrapper(const char* c_str) {
        *this = c_str;
    }

    CStringWrapper& operator=(const char* c_str) {
        if (c_str == nullptr) {
            return *this;
        }
        str = c_str;
        Rust::free_string(c_str);
        return *this;
    }

    std::string str;
};

template<typename T>
class CResult {
public:
    /// Implicit move constructor.
    /// This constructor is not fired if `R` type is `CResult`, i.e not a move constructor.
    template<typename R>
    CResult(const R& result) {
        *this = result;
    }

    template <typename R>
    CResult& operator=(const R& result) {
        code = result.code;
        if (code == OK_CODE) {
            inner = result.result;
        }
        return *this;
    }

    /// Returns an inner value.
    /// This method must be used if only `CResult::isOk` returns true,
    /// otherwise it throws an exception.
    T unwrap() {
        return *inner;
    }

    /// Returns the result value if `CResult::isOk` return true, otherwise returns a default value.
    T unwrap_or_default() {
        return inner ? *inner : T {};
    }

    /// Whether the result contains a value.
    bool isOk() const {
        return code == OK_CODE;
    }

    /// Whether the result contains an error.
    bool isErr() const {
        return !isOk();
    }

private:
    ErrorCode code;
    std::optional<T> inner;
};

using CByteArrayResultWrapper = CResult<CByteArrayWrapper>;

} // namespace TW::Rust
