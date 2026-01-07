/*

THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE

Copyright 2025 Vincent Maciejewski,  & M2 Tech
Contact:
v@m2te.ch
mayeski@gmail.com
https://www.linkedin.com/in/vmayeski/
http://m2te.ch/

Permission is hereby granted, free of charge, to any person
obtaining a copy of this software and associated documentation
files (the "Software"), to deal in the Software without
restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

https://opensource.org/licenses/MIT

*/

#include <list>
#include <map>
#include <string>
#include <iostream>
#include <cassert>
#include <thread>
#include <chrono>
#include "actors/Actor.hpp"
#include "actors/msg/Start.hpp"
#include "actors/msg/Shutdown.hpp"
#include "actors/act/Manager.hpp"
#include "actors/registry/RegistryClient.hpp"
#include "actors/remote/ZmqSender.hpp"

using namespace actors;
using namespace std;

static int set_thread_affinity(set<int> core_ids, pthread_t thread)
{
  if (core_ids.empty())
    return 0;

  cpu_set_t cpuset;
  CPU_ZERO(&cpuset);
  int num_cores = sysconf(_SC_NPROCESSORS_ONLN);

  for (auto core_id : core_ids)
  {
    if (core_id < 0 || core_id >= num_cores)
    {
      cerr << "bad core id: " << core_id << endl;
      return EINVAL;
    }
    CPU_SET(core_id, &cpuset);
  }

  auto rc = pthread_setaffinity_np(thread, sizeof(cpu_set_t), &cpuset);
  return rc;
}

Manager::Manager() {}

Manager::~Manager()
{
  // Stop registry heartbeat if running
  if (registry_client_) {
    registry_client_->stop_heartbeat();
  }

  for (auto p : thread_list)
    delete p;
}

void Manager::set_registry(const string& registry_endpoint,
                           const string& local_endpoint,
                           shared_ptr<ZmqSender> zmq_sender)
{
  zmq_sender_ = zmq_sender;
  local_endpoint_ = local_endpoint;

  // Create ActorRef to GlobalRegistry
  ActorRef registry_ref = zmq_sender_->remote_ref("GlobalRegistry", registry_endpoint);

  // Create registry client and start heartbeat
  registry_client_ = make_unique<registry::RegistryClient>(get_name(), registry_ref);
  registry_client_->start_heartbeat();
}

void Manager::init()
{
  for (auto actor : actor_list)
  {
    auto initmsg = new actors::msg::Start();
    cout << "Manager::init sending start to " << actor->get_name() << endl;
    actor->fast_send(initmsg, nullptr);
  }

  for (auto actor : actor_list)
  {
    auto t = new std::thread([actor]() { (*actor)(); });
    thread_list.push_back(t);

    if (!actor->affinity.empty())
    {
      cout << actor->get_name() << " setting affinity" << endl;
      if (set_thread_affinity(actor->affinity, t->native_handle()) != 0)
      {
        perror("could not assign affinity\n");
      }
    }

    if (actor->priority > 0)
    {
      cout << actor->get_name() << " setting priority to SCHED_FIFO " << actor->priority << endl;
      struct sched_param sp;
      sp.sched_priority = actor->priority;
      if (pthread_setschedparam(t->native_handle(), SCHED_FIFO, &sp) != 0)
      {
        perror("sched_setscheduler");
        cerr << "could not set priority for " << actor->get_name() << endl;
      }
      else
        cout << " priority set ok\n";
    }
    else
    {
      cout << actor->get_name() << " NOT setting priority " << actor->priority << endl;
    }
  }

  this->send(new msg::Start());
}

void Manager::end()
{
  for (auto t : thread_list)
  {
    if (t->joinable())
      t->join();
  }
}

void Manager::process_message(const Message *m)
{
  if (typeid(*m) == typeid(actors::msg::Start))
  {
    // Manager started
  }
  else if (typeid(*m) == typeid(actors::msg::Shutdown))
  {
    for (auto actor : actor_list)
    {
      actor->end();
      actor->fast_terminate();
      actor->terminated = true;
    }
    exit(0);
  }
}

void Manager::manage(actor_ptr actor, set<int> affinity, int priority, int priority_type)
{
  assert(actor != nullptr && "cannot manage null actor");

  if (actor->is_managed || managed_name_map.find(actor->get_name()) != managed_name_map.end())
  {
    cout << "actors already managed:\n";
    for (const auto &p : managed_name_map)
    {
      cout << p.first << endl;
    }
    assert(false && "actor with this name already managed");
  }

  // Check affinity
  for (auto core_id : affinity)
  {
    if (core_id < 0 || core_id >= sysconf(_SC_NPROCESSORS_ONLN))
    {
      cerr << "bad core id: " << core_id << endl;
      assert(false && "core id out of range");
    }
  }

  managed_name_map[actor->get_name()] = actor;
  expanded_name_map[actor->get_name()] = actor;

  actor->set_manager(this);
  actor_list.push_back(actor);

  actor->is_managed = true;
  actor->affinity = affinity;
  actor->priority = priority;
  actor->priority_type = priority_type;

  // Auto-register with GlobalRegistry if connected
  if (registry_client_ && !local_endpoint_.empty()) {
    try {
      registry_client_->register_actor(actor->get_name(), local_endpoint_);
      cout << "Manager: Registered '" << actor->get_name() << "' with GlobalRegistry" << endl;
    } catch (const registry::RegistryError& e) {
      cerr << "Manager: Failed to register '" << actor->get_name() << "': " << e.what() << endl;
    }
  }
}

map<string, size_t> Manager::get_queue_lengths() const noexcept
{
  map<string, size_t> ret;
  for (auto &[name, actor] : managed_name_map)
  {
    ret[name] = actor->queue_length();
  }
  return ret;
}

map<string, tuple<pid_t, int>> Manager::get_message_counts() const noexcept
{
  map<string, tuple<pid_t, int>> ret;
  for (auto &[name, actor] : managed_name_map)
    ret[name] = make_tuple(actor->tid, int(actor->msg_cnt));
  return ret;
}

list<string> Manager::get_managed_names() const noexcept
{
  list<string> ret;
  for (auto &[name, _] : expanded_name_map)
    ret.push_back(name);
  return ret;
}

actor_ptr Manager::get_local_actor(const string &name) const noexcept
{
  auto it = expanded_name_map.find(name);
  if (it != expanded_name_map.end())
    return it->second;
  return nullptr;
}

ActorRef Manager::get_actor_by_name(const string &name)
{
  // First check local actors
  if (auto* local = get_local_actor(name)) {
    return ActorRef(local);
  }

  // If not found locally, try GlobalRegistry
  if (registry_client_ && zmq_sender_) {
    // This will throw ActorNotFoundError or ActorOfflineError if not found/offline
    string endpoint = registry_client_->lookup(name);
    return zmq_sender_->remote_ref(name, endpoint);
  }

  // No registry connected and not found locally
  throw registry::ActorNotFoundError(name);
}

size_t Manager::total_queue_length()
{
  size_t total = 0;
  for (auto actor : actor_list)
  {
    total += actor->queue_length();
  }
  return total;
}
