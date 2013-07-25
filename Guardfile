# Add files and commands to this file, like the example:
#   watch(%r{file/path}) { `command(s)` }
#
guard 'shell' do
  watch(%r{src/.+}) { |m| `make` }
  watch('Makefile') { |m| `make` }
  watch(%r{tests/.+}) { |m| `nosetests #{m[0]}` }

  watch(%r{src/(.+).py}) do |m|
    test_filename = "tests/#{m[1]}_test.py"

    if File.exists?(test_filename)
      `nosetests #{test_filename}`
    end
  end
end
