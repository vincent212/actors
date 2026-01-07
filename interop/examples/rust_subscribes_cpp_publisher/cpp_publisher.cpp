/*
 * C++ Price Feed Publisher Example
 *
 * This C++ actor publishes market data to subscribers (C++ or Rust).
 * It demonstrates:
 * - Location transparency - uses ActorRef for all subscribers
 * - Receiving Subscribe messages from any actor
 * - Storing subscriber ActorRefs for later updates
 * - The pub/sub pattern across language boundaries
 */

#include <iostream>
#include <string>
#include <vector>
#include <unordered_map>
#include <cstring>
#include <ctime>
#include <cstdlib>
#include "actors/Actor.hpp"
#include "actors/ActorRef.hpp"
#include "InteropMessages.hpp"

/// Subscriber info - stores ActorRef and subscribed topics
struct SubscriberInfo {
    actors::ActorRef ref;
    std::vector<std::string> topics;
};

class PriceFeed : public actors::Actor {
public:
    PriceFeed() {
        // Register message handlers
        MESSAGE_HANDLER(msg::Subscribe, on_subscribe);
        MESSAGE_HANDLER(msg::Unsubscribe, on_unsubscribe);

        // Initialize prices
        prices_["AAPL"] = 150.0;
        prices_["GOOG"] = 2800.0;
        prices_["MSFT"] = 380.0;
        prices_["AMZN"] = 3400.0;

        std::cout << "[C++ Publisher] Created PriceFeed" << std::endl;
    }

    /// Publish updates to all subscribers (call periodically)
    void publish_all() {
        // Simulate price changes
        for (auto& [symbol, price] : prices_) {
            double change = (static_cast<double>(rand()) / RAND_MAX - 0.5) * 0.01;
            price *= (1.0 + change);
        }

        // Send to all subscribers
        for (const auto& [sub_name, sub_info] : subscribers_) {
            for (const auto& topic : sub_info.topics) {
                auto it = prices_.find(topic);
                if (it != prices_.end()) {
                    send_update(sub_info, topic, it->second);
                }
            }
        }
    }

private:
    std::unordered_map<std::string, SubscriberInfo> subscribers_;
    std::unordered_map<std::string, double> prices_;

    void on_subscribe(const msg::Subscribe* msg) noexcept {
        // Extract topic from fixed-size array
        std::string topic(msg->topic.data(),
            std::find(msg->topic.begin(), msg->topic.end(), '\0'));

        // Get sender from message metadata
        auto* sender = get_reply_to();
        if (!sender) {
            std::cerr << "[C++ Publisher] Subscribe with no sender!" << std::endl;
            return;
        }

        std::string sender_name = sender->get_name();

        std::cout << "[C++ Publisher] " << sender_name
                  << " subscribing to " << topic << std::endl;

        // Get or create subscriber entry with ActorRef
        auto it = subscribers_.find(sender_name);
        if (it == subscribers_.end()) {
            SubscriberInfo info;
            info.ref = actors::ActorRef(sender);  // Store ActorRef
            subscribers_[sender_name] = std::move(info);
        }
        auto& subscriber = subscribers_[sender_name];

        // Add topic if not already subscribed
        if (std::find(subscriber.topics.begin(), subscriber.topics.end(), topic)
                == subscriber.topics.end()) {
            subscriber.topics.push_back(topic);
        }

        // Send initial price update
        auto price_it = prices_.find(topic);
        if (price_it != prices_.end()) {
            send_update(subscriber, topic, price_it->second);
        }
    }

    void on_unsubscribe(const msg::Unsubscribe* msg) noexcept {
        std::string topic(msg->topic.data(),
            std::find(msg->topic.begin(), msg->topic.end(), '\0'));

        auto* sender = get_reply_to();
        if (!sender) return;

        std::string sender_name = sender->get_name();

        std::cout << "[C++ Publisher] " << sender_name
                  << " unsubscribing from " << topic << std::endl;

        auto it = subscribers_.find(sender_name);
        if (it != subscribers_.end()) {
            auto& topics = it->second.topics;
            topics.erase(std::remove(topics.begin(), topics.end(), topic), topics.end());

            if (topics.empty()) {
                subscribers_.erase(it);
            }
        }
    }

    void send_update(const SubscriberInfo& subscriber,
                     const std::string& symbol, double price) {
        auto* update = new msg::MarketUpdate();

        // Copy symbol to fixed-size array
        std::fill(update->symbol.begin(), update->symbol.end(), '\0');
        std::copy(symbol.begin(),
                  symbol.begin() + std::min(symbol.size(), update->symbol.size() - 1),
                  update->symbol.begin());

        update->price = price;
        update->timestamp = static_cast<int64_t>(std::time(nullptr)) * 1000;
        update->volume = rand() % 10000;

        // Location-transparent send - works for C++ or Rust subscribers
        subscriber.ref.send(update, this);
    }
};
