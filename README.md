## **Artemis**

In greek mythology, Artemis is the goddess of the hunt, wilderness and wild animals and twin sister of Apollo.  
In Rust, it's a GraphQL Client inspired by [apollo-client](https://github.com/apollographql/apollo-client) and [urql](https://github.com/FormidableLabs/urql), but with advanced code generation and compile time validation.
  
&nbsp;  
**THIS IS SUPER DUPER WORK IN PROGRESS! IT WILL PROBABLY NOT COMPILE WHEN YOU READ THIS!**  
Well, the badges will tell you actually, now that CI is set up.

![Linux (Stable)](https://github.com/wingertge/artemis/workflows/Linux%20(Stable)/badge.svg)
![Linux (Beta)](https://github.com/wingertge/artemis/workflows/Linux%20(Beta)/badge.svg)
![Linux (Nightly)](https://github.com/wingertge/artemis/workflows/Linux%20(Nightly)/badge.svg)

#### Changelog
##### artemis-normalized-cache
**v0.1.0-alpha.1**:  
* Improved read performance by approximately a factor of 4. We're now almost three times as fast as
`@urql/exchange-graphcache` on reads! *The write path is unaffected pending later optimizations.*

**v0.1.0-alpha.2**:  
* Further improved read performance, now approximately 5 times as fast as `@urql/exchange-graphcache`.
 *The write path is unaffected pending later optimizations.*
 
 **v0.1.0-alpha.3**:
 * Improve write performance by approximately a factor of 9, now 3.5 times as fast as `urql`. After improved
 measurements it seems read performance is actually just 1.5 times as fast as `urql` in read performance, but an apples
 to apples comparison between Rust and  JavaScript is impossible so these numbers aren't perfect.