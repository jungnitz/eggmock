#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <utility>
#include <vector>

namespace eggmock
{

struct signal
{
  uint32_t _v;

  signal() : _v( 0 ) {}
  explicit signal( uint32_t value ) : _v( value ) {}
  signal( uint32_t id, bool complemented ) : _v( id ^ ( static_cast<uint32_t>( complemented == 1 ) << 31 ) ) {}

  uint32_t id() const
  {
    return _v & ~( static_cast<uint32_t>( 1 ) << 31 );
  }
  bool is_complemented() const
  {
    return ( _v & ( static_cast<uint32_t>( 1 ) << 31 ) ) != 0;
  }
  signal complement() const
  {
    return signal( _v ^ ( static_cast<uint32_t>( 1 ) << 31 ) );
  }
};

namespace _private
{

template<class ntk_t>
signal from_ntk_sig( ntk_t const& ntk, typename ntk_t::signal const& s )
{
  return signal( ntk.node_to_index( ntk.get_node( s ) ), ntk.is_complemented( s ) );
}

template<class ntk_t>
typename ntk_t::signal to_ntk_sig( ntk_t& ntk, signal s )
{
  auto sig = ntk.make_signal( ntk.index_to_node( s.id() ) );
  if ( s.is_complemented() )
  {
    sig = ntk.create_not( sig );
  }
  return sig;
}

template<class ntk_t>
void free_ntkptr( void* )
{
  // no-op
}

template<class ntk_t>
signal create_false_ntkptr( void* state )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  return from_ntk_sig( *ntk, ntk->get_constant( false ) );
}

template<class ntk_t>
signal create_input_ntkptr( void* state, uint32_t idx )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  while ( ntk->num_pis() <= idx )
  {
    ntk->create_pi();
  }
  return from_ntk_sig<ntk_t>( *ntk, ntk->make_signal( ntk->pi_at( idx ) ) );
}

template<class ntk_t>
signal create_and_ntkptr( void* state, signal a, signal b )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  return from_ntk_sig<ntk_t>(
      *ntk,
      ntk->create_and( to_ntk_sig<ntk_t>( *ntk, a ), to_ntk_sig<ntk_t>( *ntk, b ) ) );
}

template<class ntk_t>
signal create_xor_ntkptr( void* state, signal a, signal b )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  return from_ntk_sig<ntk_t>(
      *ntk,
      ntk->create_xor( to_ntk_sig<ntk_t>( *ntk, a ), to_ntk_sig<ntk_t>( *ntk, b ) ) );
}

template<class ntk_t>
signal create_xor3_ntkptr( void* state, signal a, signal b, signal c )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  return from_ntk_sig<ntk_t>(
      *ntk,
      ntk->create_xor3( to_ntk_sig<ntk_t>( *ntk, a ), to_ntk_sig<ntk_t>( *ntk, b ), to_ntk_sig<ntk_t>( *ntk, c ) ) );
}

template<class ntk_t>
signal create_maj_ntkptr( void* state, signal a, signal b, signal c )
{
  auto ntk = reinterpret_cast<ntk_t*>( state );
  return from_ntk_sig<ntk_t>(
      *ntk,
      ntk->create_maj( to_ntk_sig<ntk_t>( *ntk, a ), to_ntk_sig<ntk_t>( *ntk, b ), to_ntk_sig<ntk_t>( *ntk, c ) ) );
}

template<class ntk_t>
void done_ntkptr( void* state, signal const* outputs, size_t n_outputs )
{
  auto ntk = static_cast<ntk_t*>( state );
  for ( size_t i = 0; i < n_outputs; i++ )
  {
    ntk->create_po( to_ntk_sig( *ntk, outputs[i] ) );
  }
}

} // namespace _private

template<class result>
struct receiver_ffi
{
  void* state = nullptr;
  void ( *free )( void* ) = nullptr;
  signal ( *create_false )( void* ) = nullptr;
  signal ( *create_input )( void*, uint32_t ) = nullptr;
  signal ( *create_and )( void*, signal, signal ) = nullptr;
  signal ( *create_xor )( void*, signal, signal ) = nullptr;
  signal ( *create_xor3 )( void*, signal, signal, signal ) = nullptr;
  signal ( *create_maj )( void*, signal, signal, signal ) = nullptr;
  result ( *done )( void*, signal const*, size_t ) = nullptr;
};

template<class ntk_t>
receiver_ffi<void> receive_into( ntk_t& ntk )
{
  return receiver_ffi<void>{ .state = reinterpret_cast<void*>( &ntk ),
                             .free = _private::free_ntkptr<ntk_t>,
                             .create_false = _private::create_false_ntkptr<ntk_t>,
                             .create_input = _private::create_input_ntkptr<ntk_t>,
                             .create_and = _private::create_and_ntkptr<ntk_t>,
                             .create_xor = _private::create_xor_ntkptr<ntk_t>,
                             .create_xor3 = _private::create_xor3_ntkptr<ntk_t>,
                             .create_maj = _private::create_maj_ntkptr<ntk_t>,
                             .done = _private::done_ntkptr<ntk_t> };
}

/// Safe wrapper around `receiver_ffi`.
template<class result>
struct receiver
{
public:
  receiver( receiver_ffi<result> ffi ) : _ffi( ffi ) {}
  ~receiver()
  {
    if ( _ffi.state )
    {
      _ffi.free( _ffi.state );
    }
  }
  receiver( receiver const& ) = delete;
  receiver( receiver&& from ) noexcept : receiver()
  {
    swap( *this, from );
  };
  receiver& operator=( receiver const& ) = delete;
  receiver& operator=( receiver&& from ) noexcept
  {
    swap( *this, from );
    return *this;
  }

  receiver_ffi<result> into_ffi()
  {
    auto ffi = _ffi;
    _ffi.state = nullptr;
    return ffi;
  }

  signal create_false()
  {
    return _ffi.create_false( _ffi.state );
  }

  signal create_input( uint32_t idx )
  {
    return _ffi.create_input( _ffi.state, idx );
  }

  signal create_and( signal a, signal b )
  {
    return _ffi.create_and( _ffi.state, a, b );
  }

  signal create_xor( signal a, signal b )
  {
    return _ffi.create_xor( _ffi.state, a, b );
  }

  signal create_xor3( signal a, signal b, signal c )
  {
    return _ffi.create_xor3( _ffi.state, a, b, c );
  }

  signal create_maj( signal a, signal b, signal c )
  {
    return _ffi.create_maj( _ffi.state, a, b, c );
  }

  result done( signal const* outputs, size_t n_outputs )
  {
    void* state = _ffi.state;
    _ffi.state = nullptr;
    return _ffi.done( state, outputs, n_outputs );
  }

  friend void swap( receiver& a, receiver& b )
  {
    using std::swap;
    swap( a._ffi, b._ffi );
  }

private:
  receiver() = default;
  receiver_ffi<result> _ffi;
};

struct rewriter_ffi
{
  void* data;
  void ( *free )( void* );
  void ( *rewrite )( void*, receiver_ffi<void> );
};

struct rewriter
{
public:
  explicit rewriter( rewriter_ffi ffi ) : _ffi( ffi ) {}
  ~rewriter()
  {
    if ( _ffi.data )
    {
      _ffi.free( _ffi.data );
    }
  }
  rewriter( rewriter const& ) = delete;
  rewriter( rewriter&& from ) = delete;
  rewriter& operator=( rewriter const& ) = delete;
  rewriter& operator=( rewriter&& from ) = delete;

  void rewrite( receiver<void> receiver )
  {
    void* data = _ffi.data;
    _ffi.data = nullptr;
    _ffi.rewrite( data, receiver.into_ffi() );
  }

private:
  rewriter_ffi _ffi;
};

namespace _private
{

template<class ntk_t, class result>
signal send_ntk_signal( ntk_t const& ntk, typename ntk_t::signal const& src_sig, receiver<result>& receiver )
{
  auto const node = ntk.get_node( src_sig );
  signal dst_sig;
  if ( ntk.visited( node ) )
  {
    dst_sig = signal( ntk.value( node ) );
  }
  else if ( ntk.is_pi( node ) )
  {
    dst_sig = receiver.create_input( ntk.pi_index( node ) );
  }
  else if ( ntk.is_constant( node ) )
  {
    dst_sig = receiver.create_false();
    if ( ntk.constant_value( node ) )
    {
      dst_sig = dst_sig.complement();
    }
  }
  else
  {
    // collect fanins, should be 3 at most
    signal fanins[3];
    ntk.foreach_fanin( node, [&]( typename ntk_t::signal const& fanin, uint32_t const index ) {
      if ( index >= 3 )
      {
        return;
      }
      fanins[index] = send_ntk_signal( ntk, fanin, receiver );
    } );
    if ( ntk.is_and( node ) )
    {
      dst_sig = receiver.create_and( fanins[0], fanins[1] );
    }
    else if ( ntk.is_xor( node ) )
    {
      dst_sig = receiver.create_xor( fanins[0], fanins[1] );
    }
    else if ( ntk.is_xor3( node ) )
    {
      dst_sig = receiver.create_xor3( fanins[0], fanins[1], fanins[2] );
    }
    else if ( ntk.is_maj( node ) )
    {
      dst_sig = receiver.create_maj( fanins[0], fanins[1], fanins[2] );
    }
    else
    {
      throw std::invalid_argument( "unexpected node type" );
    }
  }
  ntk.set_value( node, dst_sig._v );
  ntk.set_visited( node, true );
  if ( ntk.is_complemented( src_sig ) )
  {
    dst_sig = dst_sig.complement();
  }
  return dst_sig;
}

} // namespace _private

template<class ntk_t, class result>
result send_ntk( ntk_t const& ntk, receiver<result> receiver )
{
  ntk.clear_values();
  ntk.clear_visited();
  ntk.foreach_node( [&]( auto const& node ) {
    _private::send_ntk_signal( ntk, ntk.make_signal( node ), receiver );
  } );

  std::vector<signal> outputs;
  outputs.reserve( ntk.num_pos() );
  ntk.foreach_po( [&]( auto const& src_sig ) {
    signal sig = _private::send_ntk_signal( ntk, src_sig, receiver );
    outputs.emplace_back( sig );
  } );
  return receiver.done( outputs.data(), outputs.size() );
}

template<class ntk_t>
ntk_t rewrite( ntk_t const& ntk, receiver<rewriter_ffi> rcv )
{
  rewriter rw( send_ntk( ntk, std::move( rcv ) ) );
  ntk_t res;
  rw.rewrite( receive_into( res ) );
  return res;
}

} // namespace eggmock
