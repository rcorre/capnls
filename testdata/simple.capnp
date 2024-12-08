@0xf80ac8f51ec33627;

using import "dep.capnp".Dep;
using Other = import "other.capnp";
using Stuff = import "folder/stuff.capnp";

enum Thing {
   foo @0;
   bar @0;
   baz @2;
}

struct Foo {

   i @0 :Int32;
   t @1 :Thing;
   d @2 :Dep;

  struct Buz {}
}

struct Bar {
   f @0 :Foo;
   other @1 :Other.Other;
   buz @2 :Foo.Buz;
   otherNested @3 :Other.Other.Nested;
   stuff @4 :Stuff.Stuff;
}

struct Empty {}

