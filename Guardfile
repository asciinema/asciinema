notification :tmux, :color_location => 'status-right-bg'

def run(command)
  if system(command)
    n "OK", "Ble", :success
  else
    n "BAD", "Ble", :failed
  end

  nil
end

def run_test(file)
  run("PYTHONPATH=tests nosetests #{file}")
end

guard 'shell' do
  watch(%r{src/(.+)\.py$}) do |m|
    test_filename = "tests/#{m[1]}_test.py"

    if File.exists?(test_filename)
      run_test(test_filename)
    end
  end

  watch(%r{tests/.+\.py$}) { |m| run_test(m[0]) }
end
