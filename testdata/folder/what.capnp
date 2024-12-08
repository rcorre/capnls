@0xe3bfecd5dd0a4601;

using Stuff = import "stuff.capnp";

struct What{
     a @0 :Stuff.Stuff;
     b @1 :import "stuff.capnp".Stuff;
}

