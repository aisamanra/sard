# Here is one comment
module Opus::Foo
  # here is another comment
  class Bar
    # a constant
    X = 5
    # and here is a method comment
    # that is also multiline
    sig {void}
    def self.do_the_thing
      puts "hey"
    end
    # and another method
    sig {returns(Integer)}
    def whatever
      5 + 5
    end

    # here is a Ruby metaprogrammed method
    attr_reader :foo

    # here is a Sorbet metaprogrammed method
    const :bar, Integer
  end
end
