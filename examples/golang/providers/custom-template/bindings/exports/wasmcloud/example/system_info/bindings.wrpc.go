// Generated by `wit-bindgen-wrpc-go` 0.1.1. DO NOT EDIT!
package system_info

import (
	bytes "bytes"
	context "context"
	binary "encoding/binary"
	errors "errors"
	fmt "fmt"
	wrpc "github.com/wrpc/wrpc/go"
	io "io"
	slog "log/slog"
	math "math"
	sync "sync"
	atomic "sync/atomic"
)

type Kind uint8

const (
	Kind_Os   Kind = 0
	Kind_Arch Kind = 1
)

func (v Kind) String() string {
	switch v {
	case Kind_Os:
		return "OS"
	case Kind_Arch:
		return "ARCH"
	default:
		panic("invalid enum")
	}
}
func (v Kind) WriteToIndex(w wrpc.ByteWriter) (func(wrpc.IndexWriter) error, error) {
	if err := func(v uint8, w io.Writer) error {
		b := make([]byte, 2)
		i := binary.PutUvarint(b, uint64(v))
		slog.Debug("writing u8 discriminant")
		_, err := w.Write(b[:i])
		return err
	}(uint8(v), w); err != nil {
		return nil, fmt.Errorf("failed to write discriminant: %w", err)
	}
	return nil, nil
}

type Handler interface {
	// Request information about the system the provider is running on
	RequestInfo(ctx__ context.Context, kind Kind) (string, error)
	// Example export to call from the provider for testing
	Call(ctx__ context.Context) (string, error)
}

func ServeInterface(s wrpc.Server, h Handler) (stop func() error, err error) {
	stops := make([]func() error, 0, 2)
	stop = func() error {
		for _, stop := range stops {
			if err := stop(); err != nil {
				return err
			}
		}
		return nil
	}
	stop0, err := s.Serve("wasmcloud:example/system-info", "request-info", func(ctx context.Context, w wrpc.IndexWriter, r wrpc.IndexReadCloser) error {
		slog.DebugContext(ctx, "reading parameter", "i", 0)
		p0, err := func(r io.ByteReader) (v Kind, err error) {
			n, err := func(r io.ByteReader) (uint8, error) {
				var x uint8
				var s uint
				for i := 0; i < 2; i++ {
					slog.Debug("reading u8 discriminant byte", "i", i)
					b, err := r.ReadByte()
					if err != nil {
						if i > 0 && err == io.EOF {
							err = io.ErrUnexpectedEOF
						}
						return x, fmt.Errorf("failed to read u8 discriminant byte: %w", err)
					}
					if s == 7 && b > 0x01 {
						return x, errors.New("discriminant overflows an 8-bit integer")
					}
					if b < 0x80 {
						return x | uint8(b)<<s, nil
					}
					x |= uint8(b&0x7f) << s
					s += 7
				}
				return x, errors.New("discriminant overflows an 8-bit integer")
			}(r)
			if err != nil {
				return v, fmt.Errorf("failed to read discriminant: %w", err)
			}
			switch Kind(n) {
			case Kind_Os:
				return Kind_Os, nil
			case Kind_Arch:
				return Kind_Arch, nil
			default:
				return v, fmt.Errorf("unknown discriminant value %d", n)
			}
		}(r)
		if err != nil {
			return fmt.Errorf("failed to read parameter 0: %w", err)
		}
		slog.DebugContext(ctx, "calling `wasmcloud:example/system-info.request-info` handler")
		r0, err := h.RequestInfo(ctx, p0)
		if err != nil {
			return fmt.Errorf("failed to handle `wasmcloud:example/system-info.request-info` invocation: %w", err)
		}

		var buf bytes.Buffer
		writes := make(map[uint32]func(wrpc.IndexWriter) error, 1)
		write0, err := (func(wrpc.IndexWriter) error)(nil), func(v string, w io.Writer) (err error) {
			n := len(v)
			if n > math.MaxUint32 {
				return fmt.Errorf("string byte length of %d overflows a 32-bit integer", n)
			}
			if err = func(v int, w io.Writer) error {
				b := make([]byte, binary.MaxVarintLen32)
				i := binary.PutUvarint(b, uint64(v))
				slog.Debug("writing string byte length", "len", n)
				_, err = w.Write(b[:i])
				return err
			}(n, w); err != nil {
				return fmt.Errorf("failed to write string byte length of %d: %w", n, err)
			}
			slog.Debug("writing string bytes")
			_, err = w.Write([]byte(v))
			if err != nil {
				return fmt.Errorf("failed to write string bytes: %w", err)
			}
			return nil
		}(r0, &buf)
		if err != nil {
			return fmt.Errorf("failed to write result value 0: %w", err)
		}
		if write0 != nil {
			writes[0] = write0
		}
		slog.DebugContext(ctx, "transmitting `wasmcloud:example/system-info.request-info` result")
		_, err = w.Write(buf.Bytes())
		if err != nil {
			return fmt.Errorf("failed to write result: %w", err)
		}
		if len(writes) > 0 {
			var wg sync.WaitGroup
			var wgErr atomic.Value
			for index, write := range writes {
				wg.Add(1)
				w, err := w.Index(index)
				if err != nil {
					return fmt.Errorf("failed to index writer: %w", err)
				}
				write := write
				go func() {
					defer wg.Done()
					if err := write(w); err != nil {
						wgErr.Store(err)
					}
				}()
			}
			wg.Wait()
			err := wgErr.Load()
			if err == nil {
				return nil
			}
			return err.(error)
		}
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("failed to serve `wasmcloud:example/system-info.request-info`: %w", err)
	}
	stops = append(stops, stop0)
	stop1, err := s.Serve("wasmcloud:example/system-info", "call", func(ctx context.Context, w wrpc.IndexWriter, r wrpc.IndexReadCloser) error {
		slog.DebugContext(ctx, "calling `wasmcloud:example/system-info.call` handler")
		r0, err := h.Call(ctx)
		if err != nil {
			return fmt.Errorf("failed to handle `wasmcloud:example/system-info.call` invocation: %w", err)
		}

		var buf bytes.Buffer
		writes := make(map[uint32]func(wrpc.IndexWriter) error, 1)
		write0, err := (func(wrpc.IndexWriter) error)(nil), func(v string, w io.Writer) (err error) {
			n := len(v)
			if n > math.MaxUint32 {
				return fmt.Errorf("string byte length of %d overflows a 32-bit integer", n)
			}
			if err = func(v int, w io.Writer) error {
				b := make([]byte, binary.MaxVarintLen32)
				i := binary.PutUvarint(b, uint64(v))
				slog.Debug("writing string byte length", "len", n)
				_, err = w.Write(b[:i])
				return err
			}(n, w); err != nil {
				return fmt.Errorf("failed to write string byte length of %d: %w", n, err)
			}
			slog.Debug("writing string bytes")
			_, err = w.Write([]byte(v))
			if err != nil {
				return fmt.Errorf("failed to write string bytes: %w", err)
			}
			return nil
		}(r0, &buf)
		if err != nil {
			return fmt.Errorf("failed to write result value 0: %w", err)
		}
		if write0 != nil {
			writes[0] = write0
		}
		slog.DebugContext(ctx, "transmitting `wasmcloud:example/system-info.call` result")
		_, err = w.Write(buf.Bytes())
		if err != nil {
			return fmt.Errorf("failed to write result: %w", err)
		}
		if len(writes) > 0 {
			var wg sync.WaitGroup
			var wgErr atomic.Value
			for index, write := range writes {
				wg.Add(1)
				w, err := w.Index(index)
				if err != nil {
					return fmt.Errorf("failed to index writer: %w", err)
				}
				write := write
				go func() {
					defer wg.Done()
					if err := write(w); err != nil {
						wgErr.Store(err)
					}
				}()
			}
			wg.Wait()
			err := wgErr.Load()
			if err == nil {
				return nil
			}
			return err.(error)
		}
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("failed to serve `wasmcloud:example/system-info.call`: %w", err)
	}
	stops = append(stops, stop1)
	return stop, nil
}
